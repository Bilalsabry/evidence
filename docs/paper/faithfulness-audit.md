# Faithfulness audit

Verifies every cited corpus span is real source text — the primary reviewer attack on an AI-authored benchmark.

- **Missing** = not found verbatim or hyphen-insensitively in any corpus PDF — requires manual review; may be genuine fabrication OR aggressive normalization.
- **Fuzzy** = found only after hyphen/space stripping (likely benign OCR/hyphenation).

## Totals

- examples: 254
- corpus spans: 255
- exact: 249
- fuzzy: 1
- missing: 5

## Flagged spans

| example | span text (≤120 chars) | kind |
| --- | --- | --- |
| hamptonsun_reapply_swimming | reapply: after 80 minutes of swimming or sweating immediately after towel drying at least every 2 hours | missing |
| mucinex_max_daily_amount | Severe liver damage may occur if you take: more than 12 caplets in 24 hours, which is the maximum daily amount for this… | missing |
| dermfree_use | temporarily relieves pain and itching associated with:▇insect bites▇ minor burns ▇sunburn▇ minor skin irritations▇ mino… | missing |
| noxivent_indication | Noxivent™ is a vasodilator indicated to improve oxygenation and reduce the need for extracorporeal membrane oxygenation… | fuzzy |
| cmc_eyedrops_storage | Store between 15-30°C (59-86°F). Keep carton for complete product information. | missing |
| amiodarone_reserve_use | Reserve Amiodarone Hydrochloride Tablets for patients with the indicated life-threatening arrhythmias because its use i… | missing |


---

## Manual verification of flagged spans (2026-05-18)

Every flagged span was located in its source PDF via `evidence-eval author`. The
verbatim best-matching source line(s) are quoted below (line breaks shown as
`\r\n`; bullet glyphs reproduced as extracted). The audit tool flags these
because it does not collapse interior whitespace or merge hyphen/bullet/line
splits — all six are genuine label text.

### hamptonsun_reapply_swimming — Faithful-reconstruction
Source: `019b0c1a-f9f3-4756-8b24-35bf4dc4aa22.pdf`, page 1.
> `15 minutes before sun exposure • reapply:\r\n`
> `• after 80 minutes of swimming or sweating\r\n`
> `• immediately after towel drying • at least every 2\r\n`
> `hours`

Span = "reapply:" + the three `•`-bulleted lines merged and single-spaced.

### mucinex_max_daily_amount — Faithful-reconstruction
Source: `350764de-7801-b24e-e063-6394a90ab3c3.pdf`, page 1.
> `Severe liver damage may occur if you take:\r\n`
> `■ more than 12 caplets in 24 hours, which is\r\n`
> `the maximum daily amount for this product\r\n`

Header line + `■`-bulleted line merged; bullet glyph dropped.

### dermfree_use — Faithful-reconstruction
Source: `519c319a-9c43-0e63-e063-6394a90ab869.pdf`, page 1.
> `temporarily relieves pain and itching associated with:▇insect bites▇ minor burns\r\n`
> `▇sunburn▇ minor skin irritations▇ minor cuts ▇scrapes▇ rashes due to poison ivy,\r\n`
> `poison oak, and poison sumac ▇dries the oozing and weeping of poison ivy, poison oak\r\n`
> ` and poison sumac\r\n`

Exact text; the `▇` separators are PDF bullet-extraction artifacts preserved verbatim in the span.

### noxivent_indication — Faithful-reconstruction
Source: `4ff98b29-bcd3-82eb-e063-6394a90abf8c.pdf`, page 1.
> `Noxivent\r\n`
> `™ is a vasodilator indicated to improve oxygenation and reduce the need for\r\n`
> `extracorporeal membrane oxygenation in term and near-term (>34 weeks gestation)\r\n`
> `neonates with hypoxic respiratory failure associated with clinical or echocardiographic\r\n`
> `evidence of pulmonary hypertension in conjunction with ventilatory support and other\r\n`
> `appropriate agents.`

"Noxivent" + "™ ..." rejoined; flagged fuzzy only for the space before `™`.

### cmc_eyedrops_storage — Faithful-reconstruction
Source: `51a2d74f-5ff3-9921-e063-6294a90a345a.pdf`, page 1 (span id 1320).
> `  Store between 15-30°C (59-86°F).   Keep carton for complete product information.\r\n`

The storage sentence **does exist verbatim** in the source label. The earlier
quick grep failed only because the source has three interior spaces between the
two sentences (`...(59-86°F).   Keep...`) and a leading-indent; the audit tool
matches verbatim/hyphen-insensitively but does not collapse runs of spaces. The
benchmark span is the prescribed single-spaced, trimmed reconstruction of this
line, so no text change was required and the example was retained.

### amiodarone_reserve_use — Faithful-reconstruction
Source: `b8bf4aa2-abea-403d-a6d9-395caf5f87f0.pdf`, page 1.
> `Reserve Amiodarone Hydrochloride Tablets for patients with the indicated lifethreatening arrhythmias because its use is accompanied by substantial toxicity,\r\n`
> `some also life-threatening. Utilize alternative agents first. (1)\r\n`

Two lines merged; source line-break split `life-` / `threatening` into
`lifethreatening`, restored to the hyphenated form. The trailing "Utilize
alternative agents first. (1)" is outside the cited span.

Net: all 255 cited spans are faithful to source — 249 exact, 1 fuzzy, and 5
benign reconstruction artifacts (line/bullet/hyphen merges, interior-whitespace
collapse) confirmed verbatim against the source PDFs. No fabricated or
unverifiable spans remain; `cmc_eyedrops_storage` was confirmed against the
source and kept. Example count unchanged at 254. The structural results
(existence/in-context, non-overlap) do not depend on span text and are
unaffected.
