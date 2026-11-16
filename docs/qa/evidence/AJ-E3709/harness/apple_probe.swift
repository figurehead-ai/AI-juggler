// Throwaway probe: Apple's on-device Foundation Model only (SystemLanguageModel.default). Imports: Foundation, FoundationModels.
// Protocol: one JSON object per stdin line {id, system, prompt, max_tokens, temperature}; one JSON per stdout line.
import Foundation
import FoundationModels

func emit(_ o: [String: Any]) {
    if let d = try? JSONSerialization.data(withJSONObject: o), let s = String(data: d, encoding: .utf8) { print(s); fflush(stdout) }
}
@available(macOS 26.0, *)
func run(system: String, prompt: String, maxTokens: Int, temp: Double, permissive: Bool) async throws -> String {
    let model = permissive ? SystemLanguageModel(guardrails: .permissiveContentTransformations) : SystemLanguageModel.default
    let session = LanguageModelSession(model: model, instructions: system)
    let opts = GenerationOptions(temperature: temp, maximumResponseTokens: maxTokens)
    return try await session.respond(to: prompt, options: opts).content
}
@available(macOS 26.0, *)
func run_loop() async {
    let m = SystemLanguageModel.default
    emit(["hello": true, "available": "\(m.availability)"])
    while let line = readLine() {
        guard let d = line.data(using: .utf8), let j = try? JSONSerialization.jsonObject(with: d) as? [String: Any] else { continue }
        let id = j["id"] ?? ""; let sys = j["system"] as? String ?? ""; let p = j["prompt"] as? String ?? ""
        let mt = j["max_tokens"] as? Int ?? 60; let t = j["temperature"] as? Double ?? 0
        let t0 = Date()
        var refused = false, retried = false, err = "", text = ""
        do { text = try await run(system: sys, prompt: p, maxTokens: mt, temp: t, permissive: false) }
        catch let e as LanguageModelSession.GenerationError {
            switch e {
            case .guardrailViolation, .refusal:
                refused = true; retried = true
                do { text = try await run(system: sys, prompt: p, maxTokens: mt, temp: t, permissive: true); refused = false }
                catch { err = "retry: \(error)"; refused = true }
            default: err = "\(e)"
            }
        } catch { err = "\(error)" }
        emit(["id": id, "text": text, "refused": refused, "retried_permissive": retried, "error": err, "gen_ms": Date().timeIntervalSince(t0) * 1000])
    }
}
@main struct Probe { static func main() async { if #available(macOS 26.0, *) { await run_loop() } else { emit(["hello": true, "available": "NotThisOs"]) } } }
