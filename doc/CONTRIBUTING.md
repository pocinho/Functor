# Contributing to Functor

Thank you for your interest in contributing to **Functor**, a modern F#-powered text editor built with a clean domain architecture and Avalonia UI.

This document explains how to contribute effectively and consistently.

---

## 🧱 Project Architecture

Functor is structured into clear, independent subdomains:

- **Functor.Domain** — Pure, deterministic editor core (no I/O, no rendering)
- **Functor.Syntax** — Tokenization, language services, syntax metadata
- **Functor.Navigation** — Search, jump lists, symbol navigation
- **Functor.Diagnostics** — LSP diagnostics, ranges, severity
- **Functor.Rendering** — Layout engine, text surface, cursor rendering
- **Functor.App** — Avalonia MVU application

Each subdomain is independent and testable.

---

## 🧪 Testing

All domain logic must be covered by unit tests.  
Rendering and UI logic may use snapshot tests or integration tests.

---

## 🧬 Coding Guidelines

- Prefer **pure functions**.
- Avoid side effects in domain logic.
- Keep modules small and focused.
- Use **Position** for buffer coordinates.
- Use **Range** for selections and spans.
- Keep naming consistent across subdomains.
- Avoid mixing UI concerns into domain logic.

---

## 🧩 Pull Requests

Before submitting a PR:

1. Ensure the project builds.
2. Add or update tests.
3. Document new public APIs.
4. Keep commits clean and focused.
5. Reference related issues.

---

## 🗣 Communication

Open an issue for:

- architectural proposals  
- new features  
- bug reports  
- refactoring suggestions  

We welcome thoughtful discussion and design-oriented contributions.

---

## ❤️ Thank You

Your contributions help shape Functor into a modern, elegant, and powerful editor.
