# A13.6 Real Provider Invocation Evidence

- **Date:** 2026-09-10
- **Candidate SHA:** `964920e4cae824a49353f65a81972ce2070fe968` (`0.2.0-rc.1`)
- **Upstream Provider Tested:** OpenAI-Compatible Upstream (OpenRouter / `google/gemini-2.5-flash`)
- **Result:** **PASS**

## Test Cases & Results

| Case | Scenario | Command / Execution | Expected Result | Observed Result | Status |
| :--- | :--- | :--- | :--- | :--- | :---: |
| **1. Success Turn** | Valid provider & credentials configured | `openmesh-cli agent test --provider openai-compatible --base-url https://openrouter.ai/api/v1 --model google/gemini-2.5-flash` | Genuine model roundtrip returns valid response; identity preserved | `ok: true`, `model: google/gemini-2.5-flash`, latency 955-1332ms, `replyPreview: ok` | **PASS** |
| **2. Missing Credentials** | No key in file store or environment | Executed in isolated environment without key | Fails closed with informative error; 0 network requests; no phantom reply | Exit code 2, `error: API key not configured` | **PASS** |
| **3. Invalid Credentials** | Bogus API key passed to provider | Probe with invalid bearer token | Normalized provider error; credentials fully redacted | HTTP 401 Unauthorized; key redacted in output; 0 token exposure | **PASS** |
| **4. Invalid Provider URL** | Malformed / unsupported URL scheme | Probe with `ftp://invalid-scheme` | Normalized A5 validation error | Exit code 2, `error: provider endpoint must use http or https` | **PASS** |

## Secret Redaction Verification
- All test runs verified for zero secret leakage.
- Environment variables and token headers are not dumped into stdout or error messages.
