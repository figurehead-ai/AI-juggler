// AI Juggler's Apple helper (ADR-0033 §4.1, ADR-0034 §2, AJ-E3802): the system's on-device model, out of AI Juggler's process.
// Built outside cargo by scripts/build_apple_ai_helper.sh (swiftc, the system compiler only). The app never needs it to start:
// without it, the engine says "helper missing" and every AI job keeps its plain path.
//
// Rules this file keeps (a source test in features/ai/engines/apple checks them):
// - imports Foundation and FoundationModels only; no network, no files, no other process;
// - SystemLanguageModel.default only (the on-device model), never the cloud model;
// - guided output through DynamicGenerationSchema (the Command Line Tools ship no @Generable macro plugin);
// - temperature 0, greedy; one retry with the permissive guardrails on a refusal (ADR-0033 §4.1);
// - utility quality of service; exits when stdin closes, and after 60 s with nothing to do.
//
// Protocol 1: one JSON object per line each way.
//   out first: {"hello":true,"protocol":1,"available":<state>,"context_size":<int>}
//     state: available | device_not_eligible | apple_intelligence_not_enabled | model_not_ready | not_this_os | unknown
//   in:  {"id":<int>,"op":"respond","instructions":<text>,"prompt":<text>,"max_tokens":<int>,"schema":<shape>?}
//   out: {"id":<int>,"text":<text>,"finish":"stop"|"length","error":null|"refused"|"too_long"|"unavailable"|"failed"}
//   in:  {"op":"quit"}
// shape: {"type":"object","name","description"?,"properties":[{"name","description"?,"schema":<shape>}]}, or "string",
//   "bool", {"type":"integer","min","max"}, {"type":"enum","name","values"}, {"type":"array","items","min"?,"max"?}.
import Foundation
import FoundationModels

let protocolVersion = 1
let idleSeconds = 60.0

func emit(_ object: [String: Any]) {
    guard let data = try? JSONSerialization.data(withJSONObject: object),
          let line = String(data: data, encoding: .utf8) else { return }
    print(line)
    fflush(stdout)
}

/// Exits after `idleSeconds` with no request; each request moves it on.
final class IdleExit: @unchecked Sendable {
    private let queue = DispatchQueue(label: "idle")
    private var item: DispatchWorkItem?
    func arm() {
        queue.sync {
            item?.cancel()
            let next = DispatchWorkItem { exit(0) }
            item = next
            queue.asyncAfter(deadline: .now() + idleSeconds, execute: next)
        }
    }
    func hold() { queue.sync { item?.cancel(); item = nil } }
}

@available(macOS 26.0, *)
func state(_ availability: SystemLanguageModel.Availability) -> String {
    switch availability {
    case .available: return "available"
    case .unavailable(.deviceNotEligible): return "device_not_eligible"
    case .unavailable(.appleIntelligenceNotEnabled): return "apple_intelligence_not_enabled"
    case .unavailable(.modelNotReady): return "model_not_ready"
    default: return "unknown"
    }
}

@available(macOS 26.0, *)
func shape(_ json: [String: Any], _ name: String) -> DynamicGenerationSchema {
    switch json["type"] as? String ?? "string" {
    case "enum":
        return DynamicGenerationSchema(name: json["name"] as? String ?? name, description: json["description"] as? String,
                                       anyOf: (json["values"] as? [String]) ?? [])
    case "bool":
        return DynamicGenerationSchema(type: Bool.self)
    case "integer":
        let low = json["min"] as? Int ?? 0, high = json["max"] as? Int ?? 1000
        return DynamicGenerationSchema(type: Int.self, guides: [.range(low...max(low, high))])
    case "array":
        let items = json["items"] as? [String: Any] ?? ["type": "string"]
        return DynamicGenerationSchema(arrayOf: shape(items, name + "Item"), minimumElements: json["min"] as? Int,
                                       maximumElements: json["max"] as? Int)
    case "object":
        let properties = (json["properties"] as? [[String: Any]] ?? []).map { property -> DynamicGenerationSchema.Property in
            let field = property["name"] as? String ?? "field"
            return DynamicGenerationSchema.Property(name: field, description: property["description"] as? String,
                                                    schema: shape(property["schema"] as? [String: Any] ?? ["type": "string"],
                                                                  name + "_" + field))
        }
        return DynamicGenerationSchema(name: json["name"] as? String ?? name, description: json["description"] as? String,
                                       properties: properties)
    default:
        return DynamicGenerationSchema(type: String.self)
    }
}

@available(macOS 26.0, *)
func generate(_ request: [String: Any], permissive: Bool) async throws -> (String, Bool) {
    let model = permissive ? SystemLanguageModel(guardrails: .permissiveContentTransformations) : SystemLanguageModel.default
    let session = LanguageModelSession(model: model, instructions: request["instructions"] as? String ?? "")
    let limit = max(1, request["max_tokens"] as? Int ?? 60)
    let options = GenerationOptions(samplingMode: .greedy, temperature: 0, maximumResponseTokens: limit)
    let prompt = request["prompt"] as? String ?? ""
    if let root = request["schema"] as? [String: Any] {
        let schema = try GenerationSchema(root: shape(root, root["name"] as? String ?? "Answer"), dependencies: [])
        let response = try await session.respond(to: prompt, schema: schema, includeSchemaInPrompt: true, options: options)
        return (response.content.jsonString, false)
    }
    let response = try await session.respond(to: prompt, options: options)
    var cut = false
    if #available(macOS 27.0, *) { cut = response.usage.output.totalTokenCount >= limit }
    return (response.content, cut)
}

/// The protocol's error word for `error`, and whether a permissive retry may help.
@available(macOS 26.0, *)
func word(_ error: Error) -> (String, Bool) {
    if let generation = error as? LanguageModelSession.GenerationError {
        switch generation {
        case .guardrailViolation, .refusal: return ("refused", true)
        case .exceededContextWindowSize: return ("too_long", false)
        case .decodingFailure: return ("length", false)
        case .assetsUnavailable: return ("unavailable", false)
        default: return ("failed", false)
        }
    }
    return ("failed", false)
}

@available(macOS 26.0, *)
func answer(_ request: [String: Any]) async -> [String: Any] {
    var reply: [String: Any] = ["id": request["id"] ?? NSNull(), "text": "", "finish": "stop", "error": NSNull()]
    for permissive in [false, true] {
        do {
            let (text, cut) = try await generate(request, permissive: permissive)
            reply["text"] = text
            reply["finish"] = cut ? "length" : "stop"
            reply["error"] = NSNull()
            return reply
        } catch {
            let (what, retry) = word(error)
            if what == "length" {
                reply["finish"] = "length"
                reply["error"] = NSNull()
                return reply
            }
            reply["error"] = what
            if !retry { return reply }
        }
    }
    return reply
}

@available(macOS 26.0, *)
func serve() async {
    let model = SystemLanguageModel.default
    emit(["hello": true, "protocol": protocolVersion, "available": state(model.availability), "context_size": model.contextSize])
    let idle = IdleExit()
    idle.arm()
    while let line = readLine() {
        guard let data = line.data(using: .utf8),
              let request = (try? JSONSerialization.jsonObject(with: data)) as? [String: Any] else { continue }
        switch request["op"] as? String {
        case "quit":
            return
        case "respond":
            idle.hold()
            emit(await answer(request))
            idle.arm()
        default:
            emit(["id": request["id"] ?? NSNull(), "text": "", "finish": "stop", "error": "failed"])
        }
    }
}

@main
struct Helper {
    static func main() async {
        pthread_set_qos_class_self_np(QOS_CLASS_UTILITY, 0)
        if #available(macOS 26.0, *) {
            await serve()
        } else {
            emit(["hello": true, "protocol": protocolVersion, "available": "not_this_os", "context_size": 0])
        }
    }
}
