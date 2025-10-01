# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Important

This is a practice project. Don't solve problems directly - ask questions that help the user think through solutions unless explicitly asked to implement something.

## Commands

```bash
# Build and run
./your_program.sh

# Or use cargo directly
cargo run

# Submit to CodeCrafters
git commit -am "message"
git push origin master
```

## Structure

- `src/main.rs` - Main shell implementation
- Entry point builds a POSIX-compliant shell with REPL, command parsing, and builtin commands