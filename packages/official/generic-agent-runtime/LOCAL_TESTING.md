# Local Testing

## Start the runtime

```powershell
cd D:\weft-workspace\weft-plugins\generic-agent-runtime
py -3 server.py
```

## Start WEFT-core in aligned managed runtime mode

This experiment ships a local managed runtime root under:

- `runtime-root/`

Start WEFT-core against that runtime root with:

```powershell
cd D:\weft-workspace\weft-plugins\generic-agent-runtime
powershell -ExecutionPolicy Bypass -File .\run-managed-weft-core.ps1
```

Expected result:

- WEFT-core listens on `127.0.0.1:17830`
- `tool-runtime-core` is discoverable from `runtime-root/plugins/official/tool-runtime-core/`

Expected log:

```text
[generic-agent-runtime] generic-agent-runtime listening on 127.0.0.1:43133
```

## Important note for Windows

When testing with Python `urllib`, disable system proxy handling for localhost,
otherwise requests to `127.0.0.1` may be routed through a proxy and fail with
`HTTP 502` even though the service is healthy.

Use:

```python
opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
```

## Health check

```powershell
py -3 -c "import urllib.request; opener=urllib.request.build_opener(urllib.request.ProxyHandler({})); print(opener.open('http://127.0.0.1:43133/health', timeout=5).read().decode())"
```

## Plan task

```powershell
py -3 -c "import json, urllib.request; opener=urllib.request.build_opener(urllib.request.ProxyHandler({})); req=urllib.request.Request('http://127.0.0.1:43133/webhook', data=json.dumps({'action':'plan_task','data':{'task':'Summarize release notes','session_id':'demo-session','workspace_id':'D:\\\\workspace'}}).encode(), headers={'Content-Type':'application/json'}); print(opener.open(req, timeout=5).read().decode())"
```

## Run task

```powershell
py -3 -c "import json, urllib.request; opener=urllib.request.build_opener(urllib.request.ProxyHandler({})); req=urllib.request.Request('http://127.0.0.1:43133/webhook', data=json.dumps({'action':'run_task','data':{'task':'Summarize release notes','session_id':'demo-session','workspace_id':'D:\\\\workspace'}}).encode(), headers={'Content-Type':'application/json'}); print(opener.open(req, timeout=5).read().decode())"
```

## Run task with real WEFT tool bridge

Requires WEFT-core to be running locally on `127.0.0.1:17830`.

```powershell
py -3 -c "import json, urllib.request; opener=urllib.request.build_opener(urllib.request.ProxyHandler({})); req=urllib.request.Request('http://127.0.0.1:43133/webhook', data=json.dumps({'action':'run_task','data':{'task':'Read README through WEFT tool bridge','tool':'fs_read','args':{'path':'D:\\\\weft-workspace\\\\weft-plugins\\\\generic-agent-runtime\\\\README.md'}}}).encode(), headers={'Content-Type':'application/json'}); print(opener.open(req, timeout=5).read().decode())"
```

### Current local bridge setup

The local managed runtime root is configured to launch WEFT-core on
`127.0.0.1:17830` and expose `tool-runtime-core` from:

- `runtime-root/plugins/official/tool-runtime-core/`

The bridge now works end-to-end in this local setup.

## Verify task

```powershell
py -3 -c "import json, urllib.request; opener=urllib.request.build_opener(urllib.request.ProxyHandler({})); run_result={'task':'Summarize release notes','session_id':'demo-session','workspace_id':'D:\\\\workspace','loop':[{'turn':1,'stage':'observe','note':'received task and normalized intent'}],'result':{'status':'prototype_complete','summary':'ok'}}; req=urllib.request.Request('http://127.0.0.1:43133/webhook', data=json.dumps({'action':'verify_task','data':{'task':'Summarize release notes','run_result':run_result}}).encode(), headers={'Content-Type':'application/json'}); print(opener.open(req, timeout=5).read().decode())"
```

## Crystallize skill

```powershell
py -3 -c "import json, urllib.request; opener=urllib.request.build_opener(urllib.request.ProxyHandler({})); run_result={'task':'Summarize release notes','session_id':'demo-session','workspace_id':'D:\\\\workspace','loop':[{'turn':1,'stage':'observe','note':'received task and normalized intent'}],'result':{'status':'prototype_complete','summary':'ok'}}; verification={'task':'Summarize release notes','verdict':'PASS','checks':[{'name':'runtime completed loop','passed':True}],'notes':'prototype ok'}; req=urllib.request.Request('http://127.0.0.1:43133/webhook', data=json.dumps({'action':'crystallize_skill','data':{'task':'Summarize release notes','run_result':run_result,'verification':verification}}).encode(), headers={'Content-Type':'application/json'}); print(opener.open(req, timeout=5).read().decode())"
```

## Check generated outputs

- runtime state: `data/runtime-state.json`
- crystallized skill draft: `data/skill-drafts/summarize-release-notes.md`
