# Project Reference Review Checklist

Use this checklist when adding or changing a Functor project reference.

- [ ] The referenced project is owned by an inner layer or is an explicitly approved outer adapter dependency.
- [ ] The dependency direction remains inward: Domain has no Functor project references; Workspace and Rendering depend only on Domain; Application depends only on Domain, Workspace, and Rendering.
- [ ] The referenced type belongs to the project being referenced. Do not add a project reference only to reuse a convenience type.
- [ ] The dependency has a replaceable boundary, such as an Application port or an adapter contract.
- [ ] The affected behavior has a focused test in the owning project's test suite.
- [ ] The project file preserves the required F# compile order.
- [ ] The architecture tests pass, including `Functor.Tests.Architecture`.
- [ ] The platform-neutral CI matrix still runs independently for the affected test project.

For an intentional exception, document the reason, owning composition root, replacement plan,
and a corresponding architecture-test update in the pull request.
