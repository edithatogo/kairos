param(
  [Parameter(Mandatory=$true)][string]$JobId,
  [string]$PoolId = $env:AZURE_BATCH_POOL_ID,
  [string]$Image = $env:KAIRO_IMAGE,
  [string]$Scenario = $env:KAIRO_SCENARIO,
  [string]$OutputUri = $env:KAIRO_OUTPUT_URI
)

if (-not $PoolId) { throw "AZURE_BATCH_POOL_ID is required" }
if (-not $Image) { throw "KAIRO_IMAGE is required" }
if (-not $Scenario) { throw "KAIRO_SCENARIO is required" }
if (-not $OutputUri) { throw "KAIRO_OUTPUT_URI is required" }

az batch job create --id $JobId --pool-id $PoolId
az batch task create `
  --job-id $JobId `
  --task-id "$JobId-run" `
  --container-settings "imageName=$Image" `
  --command-line "kairo-ecs-cli run --scenario $Scenario" `
  --environment-settings "KAIRO_OUTPUT_URI=$OutputUri"
