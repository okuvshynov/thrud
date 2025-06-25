#!/bin/bash

# Thrud Utilization Metrics Script
# Shows CPU and GPU utilization for the last N collection rounds

DB_PATH="$HOME/.thrud/thrud.db"
DEFAULT_ROUNDS=5

# Get number of rounds from command line argument or use default
ROUNDS=${1:-$DEFAULT_ROUNDS}

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

echo "📊 Thrud Utilization Metrics (Last $ROUNDS rounds)"
echo "=================================================="
echo

# Get the last N+1 collection rounds (we need N+1 to calculate deltas for N rounds)
QUERY_ROUNDS=$((ROUNDS + 1))
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

if [ "$actual_rounds" -lt "$QUERY_ROUNDS" ]; then
    available_display_rounds=$((actual_rounds - 1))
    echo "ℹ️  Note: Requested $ROUNDS rounds, but only $available_display_rounds rounds available for display."
    echo
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

# Process each collection round with delta calculations
round_number=0
while IFS='|' read -r round_id timestamp; do
    round_number=$((round_number + 1))
    
    # Skip the first round - we use it only as baseline for delta calculations
    if [ $round_number -eq 1 ]; then
        continue
    fi
    
    echo "🕒 Collection Round: $(echo $round_id | cut -c1-8)... at $timestamp"
    echo "   ────────────────────────────────────────────────────────────"
    
    # Get GPU utilization (this is instantaneous, not cumulative)
    gpu_util=$(sqlite3 "$DB_PATH" "
    SELECT value 
    FROM metrics 
    WHERE collection_round_id = '$round_id' 
      AND name LIKE 'gpu.%.utilization'
    LIMIT 1
    ")
    
    if [ -n "$gpu_util" ]; then
        # GPU utilization is already a percentage (0.0-1.0)
        gpu_percent=$(echo "$gpu_util * 100" | bc -l 2>/dev/null || echo "scale=2; $gpu_util * 100" | bc)
        printf "   🔥 GPU Utilization:          %6.2f%%\n" "$gpu_percent"
    else
        echo "   🔥 GPU Utilization:          N/A"
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
            printf "   ⚡ Performance Cores:        %6.2f%%\n" "$perf_util"
        else
            echo "   ⚡ Performance Cores:        N/A (no delta)"
        fi
    else
        echo "   ⚡ Performance Cores:        N/A"
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
            printf "   🔋 Efficiency Cores:        %6.2f%%\n" "$eff_util"
        else
            echo "   🔋 Efficiency Cores:        N/A (no delta)"
        fi
    else
        echo "   🔋 Efficiency Cores:        N/A"
    fi
    
    # Get total metrics count for this round
    metric_count=$(sqlite3 "$DB_PATH" "
    SELECT COUNT(*) 
    FROM metrics 
    WHERE collection_round_id = '$round_id'
    ")
    
    echo "   📈 Total Metrics Collected: $metric_count"
    echo
done < "$rounds_file"

# Clean up temporary file
rm -f "$rounds_file"

# Show database summary
echo "📊 Database Summary"
echo "==================="

db_stats=$(sqlite3 "$DB_PATH" "
SELECT 
    COUNT(*) as total_metrics,
    COUNT(DISTINCT collection_round_id) as total_rounds,
    MIN(timestamp) as first_collection,
    MAX(timestamp) as last_collection
FROM metrics
")

if [ -n "$db_stats" ]; then
    total_metrics=$(echo "$db_stats" | cut -d'|' -f1)
    total_rounds=$(echo "$db_stats" | cut -d'|' -f2)
    first_collection=$(echo "$db_stats" | cut -d'|' -f3)
    last_collection=$(echo "$db_stats" | cut -d'|' -f4)
    
    echo "📊 Total Metrics: $total_metrics"
    echo "🔄 Total Rounds: $total_rounds"
    echo "🕐 First Collection: $first_collection"
    echo "🕐 Last Collection: $last_collection"
fi

# Show database size
if [ -f "$DB_PATH" ]; then
    db_size=$(ls -lh "$DB_PATH" | awk '{print $5}')
    echo "💾 Database Size: $db_size"
fi

echo
echo "💡 Usage: $0 [number_of_rounds]"
echo "   Default: $DEFAULT_ROUNDS rounds"