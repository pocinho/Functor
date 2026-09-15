# **Code Guidelines**  
*Guidelines for developing Functor*

---

## **1. Core Architectural Rules**

- **Domain code must be pure F#**  
  - No Avalonia types  
  - No I/O  
  - No mutable state  
  - No async workflows  
  - No UI dependencies  
  - Only pure functions, immutable records, and discriminated unions

- **Avalonia code must follow MVU**  
  - State lives in the **Model**, never in the View  
  - No mutable fields in UI logic  
  - No ViewModels  
  - Views must be stateless and derived from the Model

- **Rendering engine is UI‑agnostic**  
  - Produces `RenderCommand list`  
  - Must not reference Avalonia or SkiaSharp  
  - No DPI logic, no window size logic  
  - Pure layout + glyph measurement only

- **Subdomains must not reference each other directly**  
  - Communication happens through pure data types  
  - No circular dependencies  
  - No cross‑layer imports

---

## **2. Preferred F# Patterns**

- Use **modules + pure functions** by default  
- Use **discriminated unions** for:  
  - events  
  - commands  
  - tokens  
  - diagnostics  
  - navigation  
- Use **immutable records** for models  
- Use **pattern matching** instead of class hierarchies  
- Use **composition** instead of inheritance  
- Use **small interfaces** only when necessary (UI or DI)

---

## **3. Naming & Structure**

- Domain types: `Position`, `Range`, `Buffer`, `Cursor`  
- Events: `CoreEvent.*`, `SyntaxEvent.*`, `NavigationEvent.*`  
- Modules should reflect a single responsibility:  
  - `CursorLogic`  
  - `SelectionLogic`  
  - `Tokenizer`  
  - `LayoutEngine`

- Avoid “god modules” with too many responsibilities

---

## **4. What Should be Avoided**

- Mixing Domain and UI types  
- Introducing mutable state in Domain or MVU models  
- Suggesting Avalonia controls inside Domain logic  
- Creating classes where a DU or module is more appropriate  
- Adding side effects to pure functions  
- Adding rendering logic inside Avalonia views  
- Adding syntax logic inside rendering modules  
- Adding navigation logic inside syntax modules

---

## **5. What Should be Prefered**

- Pure transformations:  
  ```
  Model -> Event -> Model
  ```
- Clear separation of concerns  
- Small, composable modules  
- DU‑based event systems  
- MVU patterns in Avalonia  
- Pure rendering commands  
- Testable domain logic  
- Incremental syntax processing  
- Predictable navigation behavior

---

## **6. File Placement Rules**

- **Functor.Domain**  
  - Pure logic  
  - No UI  
  - No mutable state  

- **Functor.Syntax**  
  - Tokenization  
  - Incremental parsing  
  - Pure functions only  

- **Functor.Rendering**  
  - Layout  
  - Glyph runs  
  - Render commands  
  - No Avalonia types  

- **Functor.Avalonia**  
  - MVU  
  - Input → Domain events  
  - View composition  
  - No domain logic inside UI

---

## **7. General Guidance**

In development:

- Prefer purity, immutability, and DUs  
- Follow MVU strictly in Avalonia  
- Keep rendering decoupled  
- Keep domain logic isolated  
- Use small, focused modules  
- Suggest functional patterns over OOP  
- Maintain architectural boundaries  
- Avoid introducing cross‑layer dependencies  
- Keep code testable and deterministic

---