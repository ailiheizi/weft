param(
    [Parameter(Mandatory=$true)]
    [string]$EventPath
)

$event = Get-Content $EventPath -Encoding UTF8 -Raw | ConvertFrom-Json
$payload = $event.payload
$url = [string]$payload.url

if ($url -match "bilibili\.com" -and $url -match "/video/") {
    $title = if ($payload.title) { [string]$payload.title } else { "Bilibili video" }
    @{
        decision = "suggest"
        skill_id = "bilibili-companion"
        event_type = "active_url_changed"
        event_payload = @{
            title = $title
            url = $url
        }
    } | ConvertTo-Json -Compress
} else {
    @{ decision = "skip" } | ConvertTo-Json -Compress
}
