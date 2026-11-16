// Throwaway probe: Apple's on-device Foundation Model only (SystemLanguageModel.default). Imports: Foundation, FoundationModels.
// Build with the system compiler only:  swiftc -O -parse-as-library apple_probe.swift -o <scratch>/apple_probe
//
// Protocol 3: one JSON object per stdin line, one JSON object per stdout line.
//   request  {id, system, prompt, max_tokens, temperature, mode?, field?, field_description?, pattern?, count?}
//   response {id, text, finish: "stop"|"length", out_tokens, refused, retried_permissive, error, raw?, gen_ms}
// mode "line" (ADR-0033 v3 section 1, guided output): generation of a type with one String field, `field`, whose guide carries
//   `field_description` (and, when sent, `pattern`). The limits live in gate.py only; the runner sends them. The type is built with
//   DynamicGenerationSchema, the macro-free form of a one-field @Generable type: the Command Line Tools ship no FoundationModelsMacros
//   plugin, so @Generable/@Guide do not compile here.
// mode "pick": one Int field guided to 1...count (section 4.1: choose is guided generation of a single integer).
// no mode: v2's plain text (runner_v2.py), unchanged.
// finish is "length" when the output used every allowed token (usage, macOS 27+) or a guided answer could not be decoded (a structure
// cut by the token limit); `error` then says which.
import Foundation
import FoundationModels

func emit(_ o: [String: Any]) {
    if let d = try? JSONSerialization.data(withJSONObject: o), let s = String(data: d, encoding: .utf8) { print(s); fflush(stdout) }
}
struct Req {
    var system = "", prompt = "", mode = "text", field = "line", desc = "", pattern = ""
    var maxTokens = 60, count = 1, temp = 0.0
}
struct Out { var text: String; var outTokens: Int? }

@available(macOS 26.0, *)
func schema(_ r: Req) throws -> GenerationSchema {
    let prop: DynamicGenerationSchema
    if r.mode == "pick" {
        prop = DynamicGenerationSchema(type: Int.self, guides: [.range(1...max(1, r.count))])
    } else if !r.pattern.isEmpty {
        prop = DynamicGenerationSchema(type: String.self, guides: [.pattern(try Regex(r.pattern))])
    } else {
        prop = DynamicGenerationSchema(type: String.self)
    }
    let root = DynamicGenerationSchema(name: r.mode == "pick" ? "Pick" : "Line",
                                       properties: [DynamicGenerationSchema.Property(name: r.field, description: r.desc, schema: prop)])
    return try GenerationSchema(root: root, dependencies: [])
}
@available(macOS 26.0, *)
func outTokens<C>(_ resp: LanguageModelSession.Response<C>) -> Int? {
    if #available(macOS 27.0, *) { return resp.usage.output.totalTokenCount }
    return nil
}
@available(macOS 26.0, *)
func run(_ r: Req, permissive: Bool) async throws -> Out {
    let model = permissive ? SystemLanguageModel(guardrails: .permissiveContentTransformations) : SystemLanguageModel.default
    let session = LanguageModelSession(model: model, instructions: r.system)
    let opts = GenerationOptions(temperature: r.temp, maximumResponseTokens: r.maxTokens)
    if r.mode == "text" {
        let resp = try await session.respond(to: r.prompt, options: opts)
        return Out(text: resp.content, outTokens: outTokens(resp))
    }
    let resp = try await session.respond(to: r.prompt, schema: try schema(r), includeSchemaInPrompt: true, options: opts)
    let text = r.mode == "pick" ? String(try resp.content.value(Int.self, forProperty: r.field))
                                : try resp.content.value(String.self, forProperty: r.field)
    return Out(text: text, outTokens: outTokens(resp))
}
// -> (refusal or guardrail, a guided answer that could not be decoded, raw partial output if any)
@available(macOS 26.0, *)
func classify(_ e: Error) -> (Bool, Bool, String) {
    if let g = e as? LanguageModelSession.GenerationError {
        switch g {
        case .guardrailViolation, .refusal: return (true, false, "")
        case .decodingFailure: return (false, true, "")
        default: return (false, false, "")
        }
    }
    if #available(macOS 27.0, *) {
        if let l = e as? LanguageModelError {
            switch l {
            case .guardrailViolation, .refusal: return (true, false, "")
            default: return (false, false, "")
            }
        }
        if let p = e as? GeneratedContent.ParsingError { return (false, true, String(p.rawContent.prefix(300))) }
    }
    return (false, false, "")
}
@available(macOS 26.0, *)
func runLoop() async {
    emit(["hello": true, "protocol": 3, "guided": true, "available": "\(SystemLanguageModel.default.availability)",
          "os": ProcessInfo.processInfo.operatingSystemVersionString])
    while let line = readLine() {
        guard let d = line.data(using: .utf8), let j = try? JSONSerialization.jsonObject(with: d) as? [String: Any] else { continue }
        var r = Req()
        r.system = j["system"] as? String ?? ""; r.prompt = j["prompt"] as? String ?? ""
        r.mode = j["mode"] as? String ?? "text"; r.field = j["field"] as? String ?? "line"
        r.desc = j["field_description"] as? String ?? ""; r.pattern = j["pattern"] as? String ?? ""
        r.maxTokens = j["max_tokens"] as? Int ?? 60; r.count = j["count"] as? Int ?? 1; r.temp = j["temperature"] as? Double ?? 0
        let id = j["id"] ?? ""
        let t0 = Date()
        var refused = false, retried = false, err = "", text = "", raw = "", lengthHit = false
        var toks: Int? = nil
        func attempt(_ permissive: Bool) async -> Bool {   // true when a refusal asks for the permissive retry
            do {
                let o = try await run(r, permissive: permissive); text = o.text; toks = o.outTokens; return false
            } catch {
                let (isRefusal, isDecode, partial) = classify(error)
                if isDecode { lengthHit = true; raw = partial }
                err = "\(error)"
                return isRefusal
            }
        }
        if await attempt(false) {
            refused = true; retried = true; err = ""
            refused = await attempt(true) || !err.isEmpty
        }
        if let n = toks, n >= r.maxTokens { lengthHit = true }
        var o: [String: Any] = ["id": id, "text": text, "finish": lengthHit ? "length" : "stop", "refused": refused,
                                "retried_permissive": retried, "error": err, "gen_ms": Date().timeIntervalSince(t0) * 1000]
        o["out_tokens"] = toks ?? NSNull()
        if !raw.isEmpty { o["raw"] = raw }
        emit(o)
    }
}
@main struct Probe {
    static func main() async {
        if #available(macOS 26.0, *) { await runLoop() } else { emit(["hello": true, "protocol": 3, "available": "NotThisOs"]) }
    }
}
