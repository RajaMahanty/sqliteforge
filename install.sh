#!/usr/bin/env bash
# ============================================================================
# SQLiteForge - Installation Script for Ubuntu
# ============================================================================

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
CYAN='\033[0;36m'
YELLOW='\033[1;33m'
BOLD='\033[1m'
NC='\033[0m'

print_banner() {
    echo -e "${CYAN}${BOLD}"
    echo "  ███████╗ ██████╗ ██╗     ██╗████████╗███████╗███████╗ ██████╗ ██████╗  ██████╗ ███████╗"
    echo "  ██╔════╝██╔═══██╗██║     ██║╚══██╔══╝██╔════╝██╔════╝██╔═══██╗██╔══██╗██╔════╝ ██╔════╝"
    echo "  ███████╗██║   ██║██║     ██║   ██║   █████╗  █████╗  ██║   ██║██████╔╝██║  ███╗█████╗  "
    echo "  ╚════██║██║▄▄ ██║██║     ██║   ██║   ██╔══╝  ██╔══╝  ██║   ██║██╔══██╗██║   ██║██╔══╝  "
    echo "  ███████║╚██████╔╝███████╗██║   ██║   ███████╗██║     ╚██████╔╝██║  ██║╚██████╔╝███████╗"
    echo "  ╚══════╝ ╚══▀▀═╝ ╚══════╝╚═╝   ╚═╝   ╚══════╝╚═╝      ╚═════╝ ╚═╝  ╚═╝ ╚═════╝ ╚══════╝"
    echo -e "${NC}"
    echo -e "  ${BOLD}Installation Script v1.0${NC}"
    echo ""
}

info()    { echo -e "  ${CYAN}[INFO]${NC}    $1"; }
success() { echo -e "  ${GREEN}[OK]${NC}      $1"; }
warn()    { echo -e "  ${YELLOW}[WARN]${NC}    $1"; }
error()   { echo -e "  ${RED}[ERROR]${NC}   $1"; }

# ---- Check prerequisites ----
check_rust() {
    if command -v cargo &>/dev/null; then
        success "Rust toolchain found: $(rustc --version)"
        return 0
    fi

    warn "Rust toolchain not found. Installing via rustup..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"

    if command -v cargo &>/dev/null; then
        success "Rust toolchain installed: $(rustc --version)"
    else
        error "Failed to install Rust. Please install manually: https://rustup.rs"
        exit 1
    fi
}

check_build_deps() {
    local missing=()
    for pkg in build-essential pkg-config; do
        if ! dpkg -s "$pkg" &>/dev/null; then
            missing+=("$pkg")
        fi
    done

    if [ ${#missing[@]} -gt 0 ]; then
        info "Installing build dependencies: ${missing[*]}"
        sudo apt-get update -qq
        sudo apt-get install -y -qq "${missing[@]}"
        success "Build dependencies installed"
    else
        success "Build dependencies satisfied"
    fi
}

# ---- Build ----
build_release() {
    info "Building SQLiteForge in release mode..."
    cargo build --release 2>&1 | tail -3
    success "Build complete: target/release/sqliteforge"
    info "Binary size: $(du -h target/release/sqliteforge | cut -f1)"
}

# ---- Install ----
install_binary() {
    local install_dir="${1:-/usr/local/bin}"

    info "Installing to ${install_dir}/sqliteforge"
    sudo install -m 755 target/release/sqliteforge "${install_dir}/sqliteforge"
    success "Installed to ${install_dir}/sqliteforge"
}

create_dirs() {
    mkdir -p "${HOME}/.config/sqliteforge"
    mkdir -p "${HOME}/.local/share/sqliteforge"
    success "Created config and data directories"
}

create_default_config() {
    local config_path="${HOME}/.config/sqliteforge/config.toml"
    if [ ! -f "$config_path" ]; then
        cat > "$config_path" <<'EOF'
# SQLiteForge Configuration
theme = "catppuccin"
mode = "box"
headers = true
history = true
EOF
        success "Created default config at ${config_path}"
    else
        info "Config already exists at ${config_path}, skipping"
    fi
}

# ---- Man page ----
install_manpage() {
    local man_dir="/usr/local/share/man/man1"
    sudo mkdir -p "$man_dir"
    sudo tee "$man_dir/sqliteforge.1" > /dev/null <<'MANPAGE'
.TH SQLITEFORGE 1 "2026-06-15" "v1.0.0" "SQLiteForge Manual"
.SH NAME
sqliteforge \- A modern terminal-first SQLite client
.SH SYNOPSIS
.B sqliteforge
[\fIOPTIONS\fR] [\fIDATABASE\fR]
.SH DESCRIPTION
SQLiteForge is a modern, feature-rich terminal client for SQLite databases.
It provides syntax highlighting, SQL autocompletion, persistent history,
multiple output formats, and a database explorer.
.SH OPTIONS
.TP
\fB\-c\fR, \fB\-\-command\fR \fICOMMAND\fR
Execute SQL command and exit
.TP
\fB\-f\fR, \fB\-\-file\fR \fIFILE\fR
Read and execute SQL from file
.TP
\fB\-m\fR, \fB\-\-mode\fR \fIMODE\fR
Output mode (box, table, column, markdown, csv, json, list)
.TP
\fB\-h\fR, \fB\-\-help\fR
Print help
.TP
\fB\-V\fR, \fB\-\-version\fR
Print version
.SH DOT COMMANDS
.TP
.B .help
Show help message
.TP
.B .tables
List all tables
.TP
.B .schema [TABLE]
Show CREATE statements
.TP
.B .indices
List all indices
.TP
.B .mode [MODE]
Set output mode
.TP
.B .headers [on|off]
Toggle column headers
.TP
.B .read FILENAME
Execute SQL script file
.TP
.B .output [FILENAME]
Redirect output to file
.TP
.B .dump
Dump database as SQL
.TP
.B .preview TABLE
Preview first 20 rows
.TP
.B .quit / .exit
Exit SQLiteForge
.SH FILES
.TP
~/.config/sqliteforge/config.toml
Configuration file
.TP
~/.local/share/sqliteforge/history.db
Query history database
.SH AUTHOR
SQLiteForge Contributors
MANPAGE
    sudo mandb -q 2>/dev/null || true
    success "Installed man page (try: man sqliteforge)"
}

# ---- Shell completion ----
install_completions() {
    # Bash completion
    local bash_comp_dir="${HOME}/.local/share/bash-completion/completions"
    mkdir -p "$bash_comp_dir"
    cat > "${bash_comp_dir}/sqliteforge" <<'BASH_COMP'
_sqliteforge() {
    local cur prev opts
    COMPREPLY=()
    cur="${COMP_WORDS[COMP_CWORD]}"
    prev="${COMP_WORDS[COMP_CWORD-1]}"
    opts="--help --version --command --file --mode --version-info"

    case "${prev}" in
        -m|--mode)
            COMPREPLY=( $(compgen -W "box table column markdown csv json list" -- "${cur}") )
            return 0
            ;;
        -f|--file|-c|--command)
            COMPREPLY=( $(compgen -f -- "${cur}") )
            return 0
            ;;
    esac

    if [[ "${cur}" == -* ]]; then
        COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
    else
        COMPREPLY=( $(compgen -f -X '!*.db' -- "${cur}") $(compgen -f -X '!*.sqlite' -- "${cur}") $(compgen -f -X '!*.sqlite3' -- "${cur}") $(compgen -d -- "${cur}") )
    fi
}
complete -F _sqliteforge sqliteforge
BASH_COMP
    success "Installed bash completion"
}

# ---- Main ----
main() {
    print_banner

    local install_dir="/usr/local/bin"

    # Parse args
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --prefix)
                install_dir="$2/bin"
                shift 2
                ;;
            --build-only)
                check_rust
                check_build_deps
                build_release
                exit 0
                ;;
            --help)
                echo "Usage: $0 [OPTIONS]"
                echo ""
                echo "Options:"
                echo "  --prefix DIR      Install prefix (default: /usr/local)"
                echo "  --build-only      Build without installing"
                echo "  --help            Show this help"
                exit 0
                ;;
            *)
                error "Unknown option: $1"
                exit 1
                ;;
        esac
    done

    echo ""
    info "Starting installation..."
    echo ""

    check_rust
    check_build_deps
    build_release
    install_binary "$install_dir"
    create_dirs
    create_default_config
    install_manpage
    install_completions

    echo ""
    echo -e "  ${GREEN}${BOLD}╔════════════════════════════════════════╗${NC}"
    echo -e "  ${GREEN}${BOLD}║   SQLiteForge installed successfully!  ║${NC}"
    echo -e "  ${GREEN}${BOLD}╚════════════════════════════════════════╝${NC}"
    echo ""
    echo -e "  ${BOLD}Quick start:${NC}"
    echo -e "    ${CYAN}sqliteforge mydb.db${NC}              # Open/create a database"
    echo -e "    ${CYAN}sqliteforge :memory:${NC}             # In-memory database"
    echo -e "    ${CYAN}sqliteforge mydb.db -c '.tables'${NC} # Non-interactive"
    echo ""
    echo -e "  ${BOLD}Reload bash completion:${NC}"
    echo -e "    ${CYAN}source ~/.local/share/bash-completion/completions/sqliteforge${NC}"
    echo ""
}

# Ensure we're in the project directory
if [ ! -f "Cargo.toml" ]; then
    error "Please run this script from the sqliteforge project directory"
    exit 1
fi

main "$@"
