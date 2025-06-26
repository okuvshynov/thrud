#!/bin/bash

# Thrud Braille Utilization Chart Script
# Shows CPU and GPU utilization using Braille patterns (2 measurements per character)

DB_PATH="$HOME/.thrud/thrud.db"
DEFAULT_CHARS=8  # Number of Braille characters (each represents 2 measurements)

# Get number of Braille characters from command line argument or use default
CHARS=${1:-$DEFAULT_CHARS}
POINTS=$((CHARS * 2))  # Total data points (2 per Braille character)

# Braille dot patterns - each character represents a left and right column
# Left column uses dots 1,2,3,4 (positions ⠁⠂⠄⠈)
# Right column uses dots 5,6,7,8 (positions ⠐⠠⡀⢀)
# Combining gives us patterns for 0-4 dots in each column

# Function to get Braille character for given left and right dot counts
get_braille_char() {
    local left=$1
    local right=$2
    
    # Use case statement for reliable lookup without associative arrays
    case "$left,$right" in
        # Row 0: Left = 0 dots
        "0,0") echo " " ;;      # no dots (space for true 0%)
        "0,1") echo "⢀" ;;      # right: 1 dot
        "0,2") echo "⢠" ;;      # right: 2 dots
        "0,3") echo "⢰" ;;      # right: 3 dots  
        "0,4") echo "⢸" ;;      # right: 4 dots
        
        # Row 1: Left = 1 dot
        "1,0") echo "⡀" ;;      # left: 1 dot
        "1,1") echo "⣀" ;;      # left: 1, right: 1
        "1,2") echo "⣠" ;;      # left: 1, right: 2
        "1,3") echo "⣰" ;;      # left: 1, right: 3
        "1,4") echo "⣸" ;;      # left: 1, right: 4
        
        # Row 2: Left = 2 dots
        "2,0") echo "⡄" ;;      # left: 2 dots
        "2,1") echo "⣄" ;;      # left: 2, right: 1
        "2,2") echo "⣤" ;;      # left: 2, right: 2
        "2,3") echo "⣴" ;;      # left: 2, right: 3
        "2,4") echo "⣼" ;;      # left: 2, right: 4
        
        # Row 3: Left = 3 dots
        "3,0") echo "⡆" ;;      # left: 3 dots
        "3,1") echo "⣆" ;;      # left: 3, right: 1
        "3,2") echo "⣦" ;;      # left: 3, right: 2
        "3,3") echo "⣶" ;;      # left: 3, right: 3
        "3,4") echo "⣾" ;;      # left: 3, right: 4
        
        # Row 4: Left = 4 dots
        "4,0") echo "⡇" ;;      # left: 4 dots
        "4,1") echo "⣇" ;;      # left: 4, right: 1
        "4,2") echo "⣧" ;;      # left: 4, right: 2
        "4,3") echo "⣷" ;;      # left: 4, right: 3
        "4,4") echo "⣿" ;;      # left: 4, right: 4
        
        # Default fallback
        *) echo "⠀" ;;
    esac
}

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

# Function to convert percentage to dot count (0-4)
percentage_to_dots() {
    local pct=$1
    if [ -z "$pct" ] || [ "$pct" = "N/A" ]; then
        echo "0"
        return
    fi
    
    # Handle exact 0% as 0 dots
    if [ "$(echo "$pct == 0" | bc 2>/dev/null || echo "0")" = "1" ]; then
        echo "0"
        return
    fi
    
    # Convert percentage to 1-4 dot range for actual load
    # 0.1-25% = 1, 25-50% = 2, 50-75% = 3, 75-100% = 4
    local dots=$(echo "scale=0; ($pct / 25) + 1" | bc 2>/dev/null || echo "1")
    
    # Clamp to 1-4 range
    if [ "$dots" -gt 4 ]; then
        dots=4
    elif [ "$dots" -lt 1 ]; then
        dots=1
    fi
    
    echo "$dots"
}

# Function to format percentage for display (3 chars: " 5%" or "23%" or "00%")
format_percentage() {
    local pct=$1
    if [ -z "$pct" ] || [ "$pct" = "N/A" ]; then
        echo "N/A"
        return
    fi
    
    # Round to integer
    local rounded=$(echo "scale=0; ($pct + 0.5) / 1" | bc 2>/dev/null || echo "0")
    
    # Format with appropriate padding (3 characters total)
    printf "%2d%%" "$rounded"
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

# No initialization needed for case-based lookup

# Build the Braille charts
perf_chart=""
eff_chart=""
gpu_chart=""

# Process data in pairs (2 measurements per Braille character)
for ((i=0; i<${#perf_utils[@]}; i+=2)); do
    # Get left and right measurements
    left_idx=$i
    right_idx=$((i + 1))
    
    # Performance cores
    left_perf="${perf_utils[left_idx]:-N/A}"
    right_perf="${perf_utils[right_idx]:-N/A}"
    left_perf_dots=$(percentage_to_dots "$left_perf")
    right_perf_dots=$(percentage_to_dots "$right_perf")
    perf_char=$(get_braille_char "$left_perf_dots" "$right_perf_dots")
    perf_chart="$perf_chart$perf_char"
    
    # Efficiency cores
    left_eff="${eff_utils[left_idx]:-N/A}"
    right_eff="${eff_utils[right_idx]:-N/A}"
    left_eff_dots=$(percentage_to_dots "$left_eff")
    right_eff_dots=$(percentage_to_dots "$right_eff")
    eff_char=$(get_braille_char "$left_eff_dots" "$right_eff_dots")
    eff_chart="$eff_chart$eff_char"
    
    # GPU
    left_gpu="${gpu_utils[left_idx]:-N/A}"
    right_gpu="${gpu_utils[right_idx]:-N/A}"
    left_gpu_dots=$(percentage_to_dots "$left_gpu")
    right_gpu_dots=$(percentage_to_dots "$right_gpu")
    gpu_char=$(get_braille_char "$left_gpu_dots" "$right_gpu_dots")
    gpu_chart="$gpu_chart$gpu_char"
done

# Get the latest values for percentage display (last element in arrays)
latest_perf="${perf_utils[${#perf_utils[@]}-1]}"
latest_eff="${eff_utils[${#eff_utils[@]}-1]}"
latest_gpu="${gpu_utils[${#gpu_utils[@]}-1]}"

# Format percentages
perf_pct_str=$(format_percentage "$latest_perf")
eff_pct_str=$(format_percentage "$latest_eff")
gpu_pct_str=$(format_percentage "$latest_gpu")

# Output the compact Braille chart
echo "P:$perf_chart $perf_pct_str|E:$eff_chart $eff_pct_str|G:$gpu_chart $gpu_pct_str"

# Optional: Show timestamp and data point count on separate line
if [ "${2:-}" = "--verbose" ] || [ "${2:-}" = "-v" ]; then
    latest_time=$(sqlite3 "$DB_PATH" "SELECT timestamp FROM collection_rounds ORDER BY timestamp DESC LIMIT 1")
    data_points=${#perf_utils[@]}
    echo "📊 $data_points data points in $CHARS Braille chars, latest: $(echo "$latest_time" | cut -d'T' -f2 | cut -d'.' -f1)"
    echo "💡 Each Braille character represents 2 measurements (left+right columns)"
    echo "   Dot levels: 0=0%, 1=1-25%, 2=26-50%, 3=51-75%, 4=76-100%"
fi