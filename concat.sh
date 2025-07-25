#!/bin/bash

# ==========================
# concat.sh
# ==========================
# This script concatenates files from multiple directories into a single output file.
# Each directory's contents are preceded by a banner with a fixed title and a tree view.
# Supports whitelisting and blacklisting of file extensions.
# Runs in recursive mode by default; use -n to disable recursion.
# Appends cargo build output if the -b flag is specified.
#
# Usage:
#   ./concat.sh [-n] [-b]
#     -n: Disable recursive processing of subdirectories.
#     -b: Run 'cargo build' and append its output to the result.
#
# Configuration:
#   - Define the directories and their associated titles in the CONFIGURATION SECTION below.
#   - Specify desired file extensions in WHITELIST_EXTS and BLACKLIST_EXTS arrays.
#   - Specify folder names to exclude from the tree output in EXCLUDE_DIRS array.
#   - Optionally, define explicit source files in the SRC_FILES array.

# ==========================
# Configuration Section
# ==========================

# Array of source directories to concatenate
SRC_DIRS=(
    "./src"
)

# Array of titles corresponding to each source directory
TITLES=(
    "////// RAYGUI TEST"
)

# Array of explicit source files to concatenate (if any)
SRC_FILES=(
    Cargo.toml
)

# Whitelist of file extensions (without the dot)
WHITELIST_EXTS=(
    "py"
    "txt"
    "md"
    "MD"
    "rs"
    "toml"
    "sql"
    "txt"
    "sh"
    "md"
    "example"
    ".env"
    ".pub"
)

# Blacklist of file extensions (without the dot)
BLACKLIST_EXTS=(
    # Add more extensions as needed
)

# Array of directory names to exclude from the tree output and recursive file search
EXCLUDE_DIRS=(
    "venv"
    "node_modules"
    "__pycache__"
    "fetchers"
    "parsers"
    "target"
)

# Hardcoded path to the output file where the concatenated content will be saved
OUTPUT_FILE="./concat.txt"

# ==========================
# Function Definitions
# ==========================

usage() {
    echo "Usage: $0 [-n] [-b]"
    echo "  -n    Disable recursive processing of subdirectories."
    echo "  -b    Run 'cargo build' and append the output."
    exit 1
}

is_in_array() {
    local element="$1"
    shift
    local array=("$@")
    for e in "${array[@]}"; do
        if [[ "$e" == "$element" ]]; then
            return 0
        fi
    done
    return 1
}

generate_exclude_pattern() {
    local exclude_dirs=("$@")
    local pattern=""
    for dir in "${exclude_dirs[@]}"; do
        if [ -z "$pattern" ]; then
            pattern="$dir"
        else
            pattern="$pattern|$dir"
        fi
    done
    echo "$pattern"
}

# ==========================
# Argument Parsing
# ==========================

RECURSIVE=true
RUN_CARGO_BUILD=false
while getopts ":nb" opt; do
    case ${opt} in
        n )
            RECURSIVE=false
            ;;
        b )
            RUN_CARGO_BUILD=true
            ;;
        \? )
            echo "Invalid Option: -$OPTARG" 1>&2
            usage
            ;;
    esac
done
shift $((OPTIND -1))

# ==========================
# Validate Configuration
# ==========================

if [ "${#SRC_DIRS[@]}" -ne "${#TITLES[@]}" ]; then
    echo "Error: The number of source directories and titles do not match."
    exit 1
fi

if ! command -v tree &> /dev/null; then
    echo "Error: 'tree' command not found. Please install it to generate directory structures."
    exit 1
fi

# ==========================
# Script Execution
# ==========================

> "$OUTPUT_FILE"
EXCLUDE_PATTERN=$(generate_exclude_pattern "${EXCLUDE_DIRS[@]}")

for index in "${!SRC_DIRS[@]}"; do
    DIR="${SRC_DIRS[$index]}"
    TITLE="${TITLES[$index]}"

    if [ ! -d "$DIR" ]; then
        echo "Warning: Source directory '$DIR' does not exist or is not a directory. Skipping."
        continue
    fi

    echo "$TITLE" >> "$OUTPUT_FILE"
    echo "========================================" >> "$OUTPUT_FILE"
    echo "" >> "$OUTPUT_FILE"

    # Append Tree Output
    if [ "$RECURSIVE" = true ]; then
        [[ -n "$EXCLUDE_PATTERN" ]] && TREE_OUTPUT=$(tree "$DIR" -I "$EXCLUDE_PATTERN") || TREE_OUTPUT=$(tree "$DIR")
    else
        [[ -n "$EXCLUDE_PATTERN" ]] && TREE_OUTPUT=$(tree "$DIR" -L 1 -I "$EXCLUDE_PATTERN") || TREE_OUTPUT=$(tree "$DIR" -L 1)
    fi

    if [ $? -eq 0 ]; then
        echo "$TREE_OUTPUT" >> "$OUTPUT_FILE"
    else
        echo "Error: Failed to generate tree for directory '$DIR'." >> "$OUTPUT_FILE"
    fi

    echo "" >> "$OUTPUT_FILE"

    # Iterate Over Files
    if [ "$RECURSIVE" = true ]; then
        if [ "${#EXCLUDE_DIRS[@]}" -gt 0 ]; then
            FIND_ARGS=()
            for ex in "${EXCLUDE_DIRS[@]}"; do
                FIND_ARGS+=( -name "$ex" -o )
            done
            unset 'FIND_ARGS[${#FIND_ARGS[@]}-1]'
            FILES=$(find "$DIR" \( -type d \( "${FIND_ARGS[@]}" \) -prune \) -o -type f -print)
        else
            FILES=$(find "$DIR" -type f)
        fi
    else
        FILES=$(find "$DIR" -maxdepth 1 -type f)
    fi

    while IFS= read -r file; do
        if [ -f "$file" ]; then
            FILENAME=$(basename "$file")
            EXTENSION="${FILENAME##*.}"

            if is_in_array "$EXTENSION" "${BLACKLIST_EXTS[@]}"; then
                echo "Skipping '$file' due to blacklist."
                continue
            fi

            if [ "${#WHITELIST_EXTS[@]}" -gt 0 ]; then
                if ! is_in_array "$EXTENSION" "${WHITELIST_EXTS[@]}"; then
                    echo "Skipping '$file' as it is not in the whitelist."
                    continue
                fi
            fi

            HEADER="///////////////////////////// ${file#./}"
            echo "$HEADER" >> "$OUTPUT_FILE"
            cat "$file" >> "$OUTPUT_FILE"
            echo -e "
" >> "$OUTPUT_FILE"
        fi
    done <<< "$FILES"

    echo "" >> "$OUTPUT_FILE"
done

# ==========================
# Process Explicit Files
# ==========================
if [ "${#SRC_FILES[@]}" -gt 0 ]; then
    echo "////// EXPLICIT FILES" >> "$OUTPUT_FILE"
    echo "========================================" >> "$OUTPUT_FILE"
    echo "" >> "$OUTPUT_FILE"
    for file in "${SRC_FILES[@]}"; do
        if [ ! -f "$file" ]; then
            echo "Warning: Explicit file '$file' does not exist. Skipping."
            continue
        fi
        FILENAME=$(basename "$file")
        EXTENSION="${FILENAME##*.}"
        if is_in_array "$EXTENSION" "${BLACKLIST_EXTS[@]}"; then
            echo "Skipping '$file' due to blacklist."
            continue
        fi
        if [ "${#WHITELIST_EXTS[@]}" -gt 0 ]; then
            if ! is_in_array "$EXTENSION" "${WHITELIST_EXTS[@]}"; then
                echo "Skipping '$file' as it is not in the whitelist."
                continue
            fi
        fi
        HEADER="///////////////////////////// ${file#./}"
        echo "$HEADER" >> "$OUTPUT_FILE"
        cat "$file" >> "$OUTPUT_FILE"
        echo -e "
" >> "$OUTPUT_FILE"
    done
fi

# ==========================
# Cargo Build Section
# ==========================
if [ "$RUN_CARGO_BUILD" = true ]; then
    if ! command -v cargo &> /dev/null; then
        echo "Error: 'cargo' command not found. Cannot run 'cargo build'."
        echo -e "\nError: 'cargo' command not found." >> "$OUTPUT_FILE"
    else
        echo "Running 'cargo build' and appending output..."
        echo "" >> "$OUTPUT_FILE"
        echo "////// CARGO BUILD OUTPUT" >> "$OUTPUT_FILE"
        echo "========================================" >> "$OUTPUT_FILE"
        echo "" >> "$OUTPUT_FILE"
        
        # Execute cargo build, appending both standard output and standard error to the output file
        cargo build >> "$OUTPUT_FILE" 2>&1
        
        if [ $? -eq 0 ]; then
            echo "'cargo build' completed successfully. Output appended to '$OUTPUT_FILE'."
        else
            # This message goes to the console, the actual error from cargo is in the file
            echo "Warning: 'cargo build' encountered errors. Check '$OUTPUT_FILE' for details."
        fi
    fi
fi

echo "All specified directories have been concatenated into '$OUTPUT_FILE'."