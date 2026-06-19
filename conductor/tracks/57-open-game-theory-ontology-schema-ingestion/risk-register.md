# Track 57 Risk Register

| Risk | Impact | Mitigation |
|---|---|---|
| External research schemas drift | Parser instability | Keep fixtures versioned and source-attributed. |
| Turtle/JSON-LD parsing becomes string scraping | Incorrect ontology semantics | Use structured parsers or a constrained grammar with negative fixtures. |
| Subrepo boundary is ambiguous | Dirty nested repo state | Record whether the subrepo is nested Git or repo directory before implementation. |
