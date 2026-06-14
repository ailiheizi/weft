# Weft Browser Context Producer

Loads as an unpacked Chrome extension and produces:

- `active_url_changed`
- `reading_page_detected`

It posts events directly to the context-engine runtime at `http://127.0.0.1:43131/webhook`.
