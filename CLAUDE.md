# Claude Project Constitution (Industrial Grade)

This project is an **industrial scheduling decision-support system**
for **hot-rolled finishing lines (热轧精整机组)** in steel manufacturing.

This system is NOT:

- an automatic control system
- an optimization playground
- a generic task scheduler
- a scoring-based priority engine

The system provides **decision support only**.
**Human operators always have final control.**

---

## Highest Authority

The highest authority document of this project is:

- `spec/Claude_Dev_Master_Spec.md`

Secondary specs (implementation details):

- spec/Engine_Specs_v0.3_Integrated.md
- spec/Field_Mapping_Spec_v0.3_Integrated.md
- spec/Tauri_API_Contract_v0.3_Integrated.md
- spec/data_dictionary_v0.1.md

All code, design, refactoring, and suggestions produced by Claude
**must strictly comply** with this document.

If there is any conflict:

> **STOP and ask for clarification.**

---

## Industrial Red Lines (Must Never Be Violated)

Claude must never violate the following industrial constraints:

1. **Frozen Zone Protection**

   - Frozen materials must never be automatically adjusted or reordered.
2. **Maturity (Cooling) Constraint**

   - Immature (non-适温) materials must never enter the current-day capacity pool.
3. **Layered Urgency**

   - Urgency is a *level system (L0–L3)*, NOT a numeric score.
4. **Capacity First**

   - Capacity pool constraints always override material ordering.
5. **Explainability**

   - Every decision must provide explicit reasons.
   - Pure numeric outputs without explanation are forbidden.

---

## State Boundary Rules

- `material_state` is the **single source of truth**.
- `plan_item` is a **scenario snapshot only**.
- Claude must never modify factual state
  to “make scheduling work”.

---

## Claude Behavior Rules

Claude must assume:

- Industrial correctness > code elegance
- Business semantics > algorithmic cleverness
- Transparency > optimization

Claude must NOT:

- redesign system architecture
- merge engines into a single algorithm
- introduce global optimization logic
- simplify industrial rules “for convenience”

---


Before writing any code, Claude must always ask:

> Which module am I implementing, which spec files must I cite (Master + relevant Integrated spec), and what am I forbidden to change?
