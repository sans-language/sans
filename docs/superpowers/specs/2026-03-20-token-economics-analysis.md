# Sans Token Economics: Skeptic Analysis

**Date:** 2026-03-20
**Question:** Does a language designed for fewer tokens make economic sense?

---

## Token Reduction: Actual Numbers

| Language | vs Sans | Where Sans Wins | Where Sans Loses |
|----------|---------|-----------------|------------------|
| Go | 41% shorter | Error handling (55%), HTTP server (57%), enums (47%) | Concurrency (Go is 19% shorter) |
| Rust | 38% shorter | HTTP server (62%), error handling (48%) | — |
| TypeScript | 24% shorter | Error handling, enums | map/filter (TS 11% shorter), JSON (tie) |
| Python | 18% shorter | Error handling (35%), concurrency (22%) | map/filter, file I/O, JSON parsing (Python wins all three) |

---

## Where the Savings Come From

### vs Go (41% reduction)

```
85% from structural simplification:
  - No `fn` keyword
  - Expression bodies (f(x:I) I = x*2)
  - Implicit returns
  - ? operator for errors
  - No semicolons
  - Ternary operator
  - No package/import boilerplate

15% from short aliases:
  - I/S/B/F instead of int/string/bool/float
  - p instead of fmt.Println
  - fr instead of os.ReadFile
```

### vs Python (18% reduction)

```
53% from structural simplification
47% from short aliases

Without aliases, Sans is only ~10% shorter than Python.
```

**Key insight:** The structural design is the real innovation. The aliases are marginal.

---

## The Cost Argument (It's Dead)

### Code is only 10-15% of total session tokens

```
Typical AI coding session token breakdown:

  System prompt / instructions:     10%
  Conversation history:             30%
  File context (input):             30%
  User prompt:                       5%
  AI explanation text:              15%  ← not code
  AI generated CODE:                10%  ← this is what Sans compresses
```

A 40% reduction on 10-15% of tokens = **4-6% total session savings**.

### Dollar impact

| Scenario | Annual Savings vs Go | vs Python |
|----------|---------------------|-----------|
| 1K programs/month, Claude Opus ($75/Mt) | ~$10 | ~$51 |
| 1K programs/month, Claude Sonnet ($15/Mt) | ~$2 | ~$10 |
| 1K programs/month, GPT-4o-mini ($0.60/Mt) | ~$0.08 | ~$0.41 |
| **2027 projected (10x cheaper)** | **~$1** | **~$5** |

**The cost argument does not survive contact with arithmetic.**

---

## The Training Data Problem

| Language | Public Training Data | LLM Familiarity |
|----------|---------------------|-----------------|
| Python | 5-10 billion lines | Near-perfect |
| JavaScript/TS | 5-10 billion lines | Near-perfect |
| Go | 1-3 billion lines | Very high |
| Rust | 500M - 1B lines | High |
| **Sans** | **~12,000 lines** | **Effectively zero** |

### What this means

- LLMs generating Sans work from **zero pre-trained intuition**
- Every generation needs in-context syntax examples (costs tokens)
- Higher hallucination rate on unfamiliar syntax
- **If Sans causes even 5-10% more compile failures than Go, the retry cost wipes out all token savings**

### The retry math

```
Scenario: 100 code generations

Go (baseline):
  100 generations × 500 tokens = 50,000 tokens
  ~2% failure rate → 2 retries × 500 = 1,000 tokens
  Total: 51,000 tokens

Sans (40% fewer tokens, but 10% more failures):
  100 generations × 300 tokens = 30,000 tokens
  ~12% failure rate → 12 retries × 300 = 3,600 tokens
  Total: 33,600 tokens (still 34% cheaper)

Sans (40% fewer tokens, but 20% more failures):
  100 generations × 300 tokens = 30,000 tokens
  ~22% failure rate → 22 retries × 300 = 6,600 tokens
  Total: 36,600 tokens (only 28% cheaper — and that's before
         counting the developer's wasted time on failures)
```

The savings hold if error rates stay low. But with 12,000 lines of training data vs billions, low error rates are not guaranteed.

---

## What DOES Hold Up

### 1. Latency (Strong, but time-limited)

```
Output generation speed: ~30-50ms per token

500 tokens (Go):   15-25 seconds
300 tokens (Sans):  9-15 seconds
Savings:           6-10 seconds per generation
```

This is perceptible and real in interactive coding. **However:** inference speeds are improving ~5x per year. This advantage shrinks to nothing by 2027-2028.

### 2. Density for Reasoning (Moderate)

More code per attention span = model sees more relevant logic = better reasoning about relationships between functions.

This is real but hard to quantify. Not about fitting more in the context window (windows are already 200K-1M tokens). About **attention quality** on what's there.

### 3. Smaller Grammar = More Predictable Generation (Strong, permanent)

```
Go keywords: 25
Rust keywords: 39
Python keywords: 35
TypeScript keywords: ~63

Sans keywords: ~26 (but with a much simpler grammar)
```

Fewer keywords + fewer syntax variants + one way to do things = smaller surface area for hallucinations. **This advantage survives indefinitely** and is independent of training data or token costs.

### 4. Batteries Included = Fewer Dependency Hallucinations (Strong, permanent)

LLMs frequently hallucinate:
- Package names that don't exist
- API signatures that are wrong
- Import paths for the wrong version

Every capability built into the language is one less hallucination opportunity. This is real, permanent, and measurable.

---

## The Reframe

### What "fewer tokens" actually means (honestly)

| Claim | Reality |
|-------|---------|
| "Fewer tokens = cheaper" | Dead. Savings are pennies. Costs dropping 10x every 18 months. |
| "Fewer tokens = faster" | True today. Marginal by 2027-2028. |
| "Denser code = better reasoning" | Plausible but unproven. |
| "Simpler grammar = fewer hallucinations" | Theoretically strong. Needs empirical validation. |
| "Batteries included = fewer errors" | True and permanent. Not unique to Sans. |

### The pitch that survives

**Not this:** "Sans uses fewer tokens, saving you money on AI API costs."

**This:** "Sans is the language AI gets right on the first try."

Which means:
- Type system catches hallucinations at compile time (Correctness by Default)
- One way to do everything — AI picks the right form every time (One Obvious Way)
- Small grammar — fewer places to hallucinate wrong syntax (Small Surface Area)
- Everything built in — no hallucinated package names (Batteries Included)
- Fast compile → fast retry when AI does get it wrong (Fast Feedback Loop)
- Dense output — faster generation, more logic in context (a bonus, not the pitch)

### The metric that matters

**Stop measuring:** token count per program
**Start measuring:** % of AI generations that compile on first try

If Sans achieves 95% first-compile success rate vs Go's 90%, that's worth more than any token savings. If Sans achieves 80% vs Go's 90%, no amount of token savings makes up for it.

---

## What Should Change

1. **Website/README:** Replace "fewer tokens, lower costs" with correctness-first messaging
2. **Track first-compile success rate** as the primary metric for AI-native claims
3. **Invest in training data:** More public Sans code = better AI generation = self-reinforcing loop
4. **Short aliases stay** but pitch them as density/latency, not cost
5. **Structural simplification is the real differentiator** — no `fn`, expression bodies, `?` operator, implicit returns. These are language design wins, not token tricks.

---

## Bottom Line

Does a language designed for fewer tokens make sense?

**The cost argument: No.** Token prices are falling too fast. The savings are already pennies.

**The latency argument: Yes, for now.** 40% faster generation is real. Diminishing by 2027-2028.

**The correctness argument: Yes, permanently.** A simpler grammar with one way to do things, strong types, and batteries included is genuinely better for AI generation. But this is about *language design quality*, not *token count*.

**The honest answer:** Sans makes sense not because it uses fewer tokens, but because its design decisions (strong types, Result/Option, one canonical form, batteries included, simple grammar) make AI-generated code more likely to be correct. The token reduction is a nice side effect of good design, not the reason the design is good.
