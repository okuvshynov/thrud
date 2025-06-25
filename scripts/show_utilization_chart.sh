#!/bin/bash

# Thrud Utilization Chart Script
# Shows CPU and GPU utilization as compact Unicode bar charts

DB_PATH="$HOME/.thrud/thrud.db"
DEFAULT_POINTS=10

# Get number of data points from command line argument or use default
POINTS=${1:-$DEFAULT_POINTS}

# Unicode bar characters (from lowest to highest)
# Using 8 levels: space (0%), then ▁ ▂ ▃ ▄ ▅ ▆ ▇ █ for actual load
BAR_CHARS=(" " "▁" "▂" "▃" "▄" "▅" "▆" "▇" "█")

# Check if database exists
if [ ! -f "$DB_PATH" ]; then
    echo "❌ Database not found at: $DB_PATH"
    echo "   Run 'cargo run --bin thrud-collector' first to create the database."
    exit 1
fi

# Check if sqlite3 is available
if ! command -v sqlite3 &> /dev/null; then
    echo "❌ sqlite3 command not found. Please install SQLite3."
    exit 1
fi

# Function to convert percentage to bar character index (0-8)
percentage_to_bar_index() {
    local pct=$1
    if [ -z "$pct" ] || [ "$pct" = "N/A" ]; then
        echo "0"
        return
    fi
    
    # Handle true 0% as space character (index 0)
    if [ "$(echo "$pct == 0" | bc 2>/dev/null || echo "0")" = "1" ]; then
        echo "0"
        return
    fi
    
    # Convert percentage to 1-8 range for actual load
    # 0.1-12.5% = 1, 12.5-25% = 2, 25-37.5% = 3, etc.
    local index=$(echo "scale=0; ($pct / 12.5) + 1" | bc 2>/dev/null || echo "1")
    
    # Clamp to 1-8 range
    if [ "$index" -gt 8 ]; then
        index=8
    elif [ "$index" -lt 1 ]; then
        index=1
    fi
    
    echo "$index"
}

# Function to format percentage for display (4 chars with padding dots)
format_percentage() {
    local pct=$1
    if [ -z "$pct" ] || [ "$pct" = "N/A" ]; then
        echo ".N/A"
        return
    fi
    
    # Round to integer
    local rounded=$(echo "scale=0; ($pct + 0.5) / 1" | bc 2>/dev/null || echo "0")
    
    # Format with dot padding to distinguish from chart spaces
    if [ "$rounded" -lt 10 ]; then
        printf "..%s%%" "$rounded"
    elif [ "$rounded" -lt 100 ]; then
        printf ".%s%%" "$rounded"
    else
        printf "%s%%" "$rounded"
    fi
}

# Get the last N+1 collection rounds (we need N+1 to calculate deltas for N rounds)
QUERY_ROUNDS=$((POINTS + 1))
collection_rounds=$(sqlite3 "$DB_PATH" "
SELECT id, timestamp 
FROM collection_rounds 
ORDER BY timestamp DESC 
LIMIT $QUERY_ROUNDS
")

if [ -z "$collection_rounds" ]; then
    echo "❌ No collection rounds found in database."
    echo "   Run 'cargo run --bin thrud-collector' to start collecting metrics."
    exit 1
fi

# Count the actual number of rounds we got
actual_rounds=$(echo "$collection_rounds" | wc -l | tr -d ' ')
if [ "$actual_rounds" -lt 2 ]; then
    echo "❌ Need at least 2 collection rounds to calculate utilization deltas."
    echo "   Found only $actual_rounds round(s). Run 'cargo run --bin thrud-collector' longer."
    exit 1
fi

# Create temporary file to store rounds in chronological order (oldest first)
rounds_file=$(mktemp)
# Use tail -r on macOS/BSD or awk for portable reverse
if command -v tail >/dev/null 2>&1 && tail -r /dev/null >/dev/null 2>&1; then
    echo "$collection_rounds" | tail -r > "$rounds_file"
else
    # Fallback using awk for systems without tail -r
    echo "$collection_rounds" | awk '{a[NR]=$0} END {for(i=NR;i>0;i--) print a[i]}' > "$rounds_file"
fi

# Arrays to store utilization data
perf_utils=()
eff_utils=()
gpu_utils=()

# Process each collection round with delta calculations
round_number=0
while IFS='|' read -r round_id timestamp; do
    round_number=$((round_number + 1))
    
    # Skip the first round - we use it only as baseline for delta calculations
    if [ $round_number -eq 1 ]; then
        continue
    fi
    
    # Get GPU utilization (this is instantaneous, not cumulative)
    gpu_util=$(sqlite3 "$DB_PATH" "
    SELECT value 
    FROM metrics 
    WHERE collection_round_id = '$round_id' 
      AND name LIKE 'gpu.%.utilization'
    LIMIT 1
    ")
    
    if [ -n "$gpu_util" ]; then
        # GPU utilization is already a percentage (0.0-1.0), convert to 0-100
        gpu_percent=$(echo "$gpu_util * 100" | bc -l 2>/dev/null || echo "scale=2; $gpu_util * 100" | bc)
        gpu_utils+=("$gpu_percent")
    else
        gpu_utils+=("N/A")
    fi
    
    # Get previous round ID for delta calculations
    prev_round_id=$(sed -n "$((round_number - 1))p" "$rounds_file" | cut -d'|' -f1)
    
    # Calculate Performance cores utilization delta
    perf_delta=$(sqlite3 "$DB_PATH" "
    WITH current AS (
        SELECT 
            SUM(CASE WHEN name = 'cpu.performance.total_ticks' THEN CAST(value AS INTEGER) ELSE 0 END) as total,
            SUM(CASE WHEN name = 'cpu.performance.idle_ticks' THEN CAST(value AS INTEGER) ELSE 0 END) as idle
        FROM metrics 
        WHERE collection_round_id = '$round_id' 
          AND name IN ('cpu.performance.total_ticks', 'cpu.performance.idle_ticks')
    ),
    previous AS (
        SELECT 
            SUM(CASE WHEN name = 'cpu.performance.total_ticks' THEN CAST(value AS INTEGER) ELSE 0 END) as total,
            SUM(CASE WHEN name = 'cpu.performance.idle_ticks' THEN CAST(value AS INTEGER) ELSE 0 END) as idle
        FROM metrics 
        WHERE collection_round_id = '$prev_round_id' 
          AND name IN ('cpu.performance.total_ticks', 'cpu.performance.idle_ticks')
    )
    SELECT 
        (current.total - previous.total) as delta_total,
        (current.idle - previous.idle) as delta_idle
    FROM current, previous
    ")
    
    if [ -n "$perf_delta" ]; then
        perf_delta_total=$(echo "$perf_delta" | cut -d'|' -f1)
        perf_delta_idle=$(echo "$perf_delta" | cut -d'|' -f2)
        
        if [ "$perf_delta_total" -gt 0 ] 2>/dev/null; then
            perf_delta_active=$((perf_delta_total - perf_delta_idle))
            perf_util=$(echo "scale=2; $perf_delta_active * 100 / $perf_delta_total" | bc)
            perf_utils+=("$perf_util")
        else
            perf_utils+=("N/A")
        fi
    else
        perf_utils+=("N/A")
    fi
    
    # Calculate Efficiency cores utilization delta
    eff_delta=$(sqlite3 "$DB_PATH" "
    WITH current AS (
        SELECT 
            SUM(CASE WHEN name = 'cpu.efficiency.total_ticks' THEN CAST(value AS INTEGER) ELSE 0 END) as total,
            SUM(CASE WHEN name = 'cpu.efficiency.idle_ticks' THEN CAST(value AS INTEGER) ELSE 0 END) as idle
        FROM metrics 
        WHERE collection_round_id = '$round_id' 
          AND name IN ('cpu.efficiency.total_ticks', 'cpu.efficiency.idle_ticks')
    ),
    previous AS (
        SELECT 
            SUM(CASE WHEN name = 'cpu.efficiency.total_ticks' THEN CAST(value AS INTEGER) ELSE 0 END) as total,
            SUM(CASE WHEN name = 'cpu.efficiency.idle_ticks' THEN CAST(value AS INTEGER) ELSE 0 END) as idle
        FROM metrics 
        WHERE collection_round_id = '$prev_round_id' 
          AND name IN ('cpu.efficiency.total_ticks', 'cpu.efficiency.idle_ticks')
    )
    SELECT 
        (current.total - previous.total) as delta_total,
        (current.idle - previous.idle) as delta_idle
    FROM current, previous
    ")
    
    if [ -n "$eff_delta" ]; then
        eff_delta_total=$(echo "$eff_delta" | cut -d'|' -f1)
        eff_delta_idle=$(echo "$eff_delta" | cut -d'|' -f2)
        
        if [ "$eff_delta_total" -gt 0 ] 2>/dev/null; then
            eff_delta_active=$((eff_delta_total - eff_delta_idle))
            eff_util=$(echo "scale=2; $eff_delta_active * 100 / $eff_delta_total" | bc)
            eff_utils+=("$eff_util")
        else
            eff_utils+=("N/A")
        fi
    else
        eff_utils+=("N/A")
    fi
    
done < "$rounds_file"

# Clean up temporary file
rm -f "$rounds_file"

# Build the chart output
perf_chart=""
eff_chart=""
gpu_chart=""

# Generate bar charts (reverse order to show most recent on the right)
for ((i=${#perf_utils[@]}-1; i>=0; i--)); do
    # Performance cores
    perf_pct="${perf_utils[i]}"
    perf_idx=$(percentage_to_bar_index "$perf_pct")
    perf_chart="${BAR_CHARS[perf_idx]}$perf_chart"
    
    # Efficiency cores
    eff_pct="${eff_utils[i]}"
    eff_idx=$(percentage_to_bar_index "$eff_pct")
    eff_chart="${BAR_CHARS[eff_idx]}$eff_chart"
    
    # GPU
    gpu_pct="${gpu_utils[i]}"
    gpu_idx=$(percentage_to_bar_index "$gpu_pct")
    gpu_chart="${BAR_CHARS[gpu_idx]}$gpu_chart"
done

# Get the latest values for percentage display (last element in arrays)
latest_perf="${perf_utils[${#perf_utils[@]}-1]}"
latest_eff="${eff_utils[${#eff_utils[@]}-1]}"
latest_gpu="${gpu_utils[${#gpu_utils[@]}-1]}"

# Format percentages
perf_pct_str=$(format_percentage "$latest_perf")
eff_pct_str=$(format_percentage "$latest_eff")
gpu_pct_str=$(format_percentage "$latest_gpu")

# Output with safe separator (use regular dot instead of middle dot)
echo "P:$perf_chart.$perf_pct_str|E:$eff_chart.$eff_pct_str|G:$gpu_chart.$gpu_pct_str"

# Optional: Show timestamp and data point count on separate line
if [ "${2:-}" = "--verbose" ] || [ "${2:-}" = "-v" ]; then
    latest_time=$(sqlite3 "$DB_PATH" "SELECT timestamp FROM collection_rounds ORDER BY timestamp DESC LIMIT 1")
    data_points=${#perf_utils[@]}
    echo "📊 $data_points data points, latest: $(echo "$latest_time" | cut -d'T' -f2 | cut -d'.' -f1)"
fi