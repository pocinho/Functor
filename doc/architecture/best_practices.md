# **Functor Development Best Practices**  
*A reference document for consistent, principled, and scalable development*

---

## **Purpose**

This document defines the architectural, functional, and OOP boundaries that guide development across all Functor subdomains.  
It ensures:

- predictable behavior  
- maintainable code  
- strict separation of concerns  
- SOLID‑aligned OOP usage where appropriate  
- idiomatic functional F# in core logic  
- consistent MVU patterns in Avalonia  

---

## **1. Architectural Principles**

### **1.1 Domain Purity (Core Rule)**  
The **Domain** layer must remain:

- **pure** (no I/O, no rendering, no mutable state)  
- **deterministic**  
- **side‑effect free**  
- **fully testable**  

Allowed:
- pure functions  
- discriminated unions  
- immutable records  
- domain events  
- domain models  

Forbidden:
- async workflows  
- mutable fields  
- Avalonia types  
- SkiaSharp types  
- file I/O  
- timers  
- logging  

> If a function cannot be tested without a UI or environment, it does **not** belong in the Domain.

---

## **1.2 Subdomain Boundaries**

Each subdomain has a strict responsibility:

- **Functor.Domain** — editing logic, buffer manipulation, cursor movement  
- **Functor.Syntax** — tokenization, syntax metadata, incremental parsing  
- **Functor.Navigation** — search, jump lists, symbol navigation  
- **Functor.Diagnostics** — LSP diagnostics, severity mapping  
- **Functor.Rendering** — layout engine, glyph runs, viewport slicing  
- **Functor.Avalonia** — Avalonia MVU application, input handling, UI composition  

Rules:

- Subdomains **never** reference each other directly.  
- Communication happens through **events**, **models**, and **pure data types**.  
- Rendering never touches Domain logic.  
- Domain never touches UI types.

---

## **2. Functional F# Principles**

### **2.1 Immutability First**
All domain types must be immutable.  
Use `with` expressions for updates.

### **2.2 Pure Transformations**
Every domain function must follow:

```
Model -> Event -> Model
```

No side effects.  
No external dependencies.

### **2.3 Prefer Functions Over Classes**
Use modules + functions unless:

- you need polymorphism  
- you need dependency injection  
- you need a stateful UI component (Avalonia only)

### **2.4 Discriminated Unions for Behavior**
Use DUs for:

- editor events  
- syntax tokens  
- navigation results  
- diagnostics severity  
- rendering commands  

Avoid class hierarchies unless absolutely necessary.

---

## **3. SOLID Principles in F#**

### **3.1 Single Responsibility**
Every module must do one thing well.

Examples:

- `CursorLogic` → cursor movement  
- `SelectionLogic` → selection expansion  
- `Tokenizer` → tokenization only  
- `LayoutEngine` → glyph measurement only  

### **3.2 Open/Closed**
Extend behavior using:

- new DU cases  
- new modules  
- new event handlers  

Never modify existing logic unless fixing bugs.

### **3.3 Liskov Substitution**
Avoid inheritance in Domain.  
Use interfaces only in UI or integration layers.

### **3.4 Interface Segregation**
Prefer small interfaces:

- `IGlyphMeasurer`  
- `ITextSurface`  
- `ILanguageDefinition`  

Avoid “god interfaces”.

### **3.5 Dependency Inversion**
Domain depends on **abstractions**, not implementations.

Rendering depends on:

- `IGlyphMeasurer`  
- `ITextSurface`  

Syntax depends on:

- `ILanguageDefinition`  

---

## **4. Avalonia MVU Best Practices**

### **4.1 MVU Rule**
State lives in the **Model**, not the View.

Forbidden in MVU:

- mutable fields  
- storing UI elements in the model  
- storing domain models inside Avalonia controls  

### **4.2 ViewModel-Free Architecture**
Functor.Avalonia uses pure MVU — no ViewModels.

### **4.3 Rendering Separation**
Rendering engine produces:

```
RenderCommands list
```

Avalonia consumes them.

Rendering engine must not:

- know about Avalonia  
- know about SkiaSharp  
- know about DPI  
- know about window size  

### **4.4 Input Handling**
Avalonia input → translated → Domain events.

Example:

```
KeyDown -> CoreEvent.MoveCursor(Direction.Left)
PointerPressed -> CoreEvent.SetCursor(Position)
```

UI never manipulates Domain state directly.

---

## **5. Testing Standards**

### **5.1 Domain Tests**
Every domain function must have tests.

### **5.2 Rendering Tests**
Use snapshot tests for:

- glyph runs  
- layout  
- viewport slicing  

### **5.3 Syntax Tests**
Tokenizers must be tested with:

- incremental updates  
- large buffers  
- malformed input  

---

## **6. Naming & Consistency Rules**

### **6.1 Position & Range**
Always use:

- `Position` for coordinates  
- `Range` for spans  

Never use raw integers.

### **6.2 Modules**
Module names must reflect behavior:

- `CursorLogic`  
- `SelectionLogic`  
- `LayoutEngine`  
- `SyntaxModel`  

### **6.3 Events**
All events follow:

```
CoreEvent.*
SyntaxEvent.*
NavigationEvent.*
DiagnosticsEvent.*
```

---

## **7. Anti-Patterns to Avoid**

### ❌ Mixing Domain & UI types  
### ❌ Mutable state in Domain  
### ❌ Passing Avalonia controls into logic  
### ❌ Using classes where DUs are better  
### ❌ God modules with too many responsibilities  
### ❌ Rendering logic inside Avalonia code-behind  
### ❌ Tokenizers that depend on viewport  
### ❌ Navigation that depends on syntax colors  

---

## **8. Long-Term Architectural Guarantees**

Following this document ensures:

- Functor remains portable  
- Domain stays pure  
- Rendering stays fast  
- Syntax stays incremental  
- Navigation stays predictable  
- Avalonia stays clean  
- MVU stays simple  
- Future agentic integration (MCP) remains trivial  

---