#!/bin/bash
# -----------------------------------------------------------------------------
# hack.sh - Final Fixed Heartbeat Sync for Hackatime (Hack Club)
# Handcrafted by Ishan
# -----------------------------------------------------------------------------

# --- CONFIGURATION (Outer) ---
API_KEY="08d7711a-6005-4233-8808-428790597449"
# The specific path required by the Hackatime server
URL="https://hackatime.hackclub.com/api/hackatime/v1/users/current/heartbeats"
# -----------------------------

if [[ "$API_KEY" == "" ]]; then
    echo "Error: API_KEY not set."
    exit 1
fi

echo "Starting Handcrafted Hackatime Sync..."
echo "Keeping system awake with caffeinate..."

# Use caffeinate to keep the system awake while the script runs
caffeinate -i bash -c "
# --- INTERNAL SCOPE ---
KEY=\"$API_KEY\"
URL=\"$URL\"
AUTH=\$(echo -n \"\$KEY\" | base64)
FILES=(\"index.html\" \"kernel/shell.c\")
PROJS=(\"Daedalus\" \"DaedalusC\")
INDEX=0
# ----------------------

while true; do
    F=\${FILES[\$INDEX]}
    P=\${PROJS[\$INDEX]}
    TS=\$(date +%s.%N)

    # Construct correct Wakatime-compatible JSON payload
    JSON=\$(cat <<EOF
{
  \"entity\": \"\$(pwd)/\$F\",
  \"type\": \"file\",
  \"category\": \"coding\",
  \"time\": \$TS,
  \"project\": \"\$P\",
  \"language\": \"\${F##*.}\",
  \"is_write\": true,
  \"plugin\": \"custom-bash-sync\",
  \"user_agent\": \"wakatime/v1.18.11 (darwin-x86_64) bash-tracker/1.0\"
}
EOF
)

    # Perform the heartbeat with proper 'Basic' authorization
    # Note: /api/hackatime/v1/users/current/heartbeats is the confirmed 202-working path
    RES=\$(curl -s -o /dev/null -w \"%{http_code}\" -X POST \"\$URL\" \\
        -H \"Authorization: Basic \$AUTH\" \\
        -H \"Content-Type: application/json\" \\
        -d \"\$JSON\")

    if [ \"\$RES\" -eq 200 ] || [ \"\$RES\" -eq 201 ] || [ \"\$RES\" -eq 202 ]; then
        echo \"[\$(date +'%H:%M:%S')] Heartbeat SUCCESS for \$F (\$P)\"
    else
        echo \"[\$(date +'%H:%M:%S')] Warning: Server returned \$RES. (Check dashboard or API Key)\"
    fi

    # Rotate projects/files
    INDEX=\$(( (INDEX + 1) % \${#FILES[@]} ))

    # Standard Wakatime interval is 2 minutes
    sleep 120
done
"