# frontends/narrator/

Vol. V Ch. 9 §9.1, implementing Vol. I's first directive: describe committed state,
never create it. Consumes the perspective-filtered observation stream; context is
committed state only; the rules ride in the cached system prompt; output terminates at
presentation and NOWHERE else.

**The degradation contract is law:** any model failure — missing package, missing
credentials, timeout, refusal — drops the session to the deterministic plain renderer,
permanently, without a simulation hiccup (§9.1). The POC proved this pattern in
production; this implementation keeps it.
