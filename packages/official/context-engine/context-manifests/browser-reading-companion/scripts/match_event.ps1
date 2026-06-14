param(
    [Parameter(Mandatory=$true)]
    [string]$EventPath
)

$event = Get-Content $EventPath -Encoding UTF8 -Raw | ConvertFrom-Json
$payload = $event.payload

$articleText = [string]$payload.article_text
$hasText = $articleText -and $articleText.Length -ge 200
$longDwell = $payload.dwell_seconds -ge 30

if ($hasText -or $longDwell) {
    $title = if ($payload.title) { [string]$payload.title } else { "web page" }
    @{
        decision = "suggest"
        skill_id = "browser-reading-companion"
        event_type = "reading_page_detected"
        event_payload = @{
            title = $title
            url = if ($payload.url) { [string]$payload.url } else { "" }
        }
    } | ConvertTo-Json -Compress
} else {
    @{ decision = "skip" } | ConvertTo-Json -Compress
}
