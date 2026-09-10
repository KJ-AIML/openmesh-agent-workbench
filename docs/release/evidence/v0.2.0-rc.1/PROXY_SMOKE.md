# A13.7 HTTP Proxy Adapter Reality Check Evidence

- **Date:** 2026-09-10
- **Candidate SHA:** `964920e4cae824a49353f65a81972ce2070fe968` (`0.2.0-rc.1`)
- **Runtime:** OpenMesh built-in HTTP proxy adapter (`openmesh-cli proxy serve`)
- **Result:** **PASS**

## Verified Behaviors

| Verification Item | Target / Command | Observed Result | Status |
| :--- | :--- | :--- | :---: |
| **1. In-Process Independence** | Agent Engine invocation while proxy listener stopped | In-process provider runtime executes turns directly without proxy listener running | **PASS** |
| **2. Proxy Health Endpoint** | `GET http://127.0.0.1:8318/healthz` | Status 200: `{"bindHost":"127.0.0.1","port":8318,"runtime":"openmesh-built-in","status":"ok"}` | **PASS** |
| **3. Model Catalog Endpoint** | `GET http://127.0.0.1:8318/v1/models` with client bearer auth | Status 200: `{"data":[{"capabilities":["chat","responses","embeddings"],"id":"google/gemini-2.5-flash","object":"model","owned_by":"openmesh-upstream"}],"object":"list"}` | **PASS** |
| **4. Live Chat Completion** | `POST http://127.0.0.1:8318/v1/chat/completions` | Received genuine upstream response: `openmesh-proxy-verified` via Google Gemini on OpenRouter | **PASS** |
| **5. Authentication Gate** | Unauthenticated requests to `/v1/models` or `/v1/chat/completions` | Correctly rejected with HTTP 401 Unauthorized | **PASS** |
| **6. Listener Lifecycle** | Process termination / unbind | SIGINT cleanly terminated server; subsequent request to port failed with Connection Refused | **PASS** |
| **7. Anti-Loop Protection** | Chat pointed at local proxy URL (`catalog.rs`) | `looks_like_builtin_proxy_url` intercepts loopback proxy URLs and resolves upstream in-process, preventing recursive HTTP loops | **PASS** |
| **8. Config Persistence** | Stopping listener | Proxy configuration lives independently in YAML storage; unbinding port does not erase Chat provider configuration | **PASS** |
