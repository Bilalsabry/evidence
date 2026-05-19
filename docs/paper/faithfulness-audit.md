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

Each flagged span was checked against its source PDF via `evidence-eval author`:

| example | verdict |
|---|---|
| amiodarone_reserve_use | **Faithful.** Real label text (id 1460+). Flagged only for an inserted hyphen (`lifethreatening`→`life-threatening`) + cross-line merge. |
| mucinex_max_daily_amount | **Faithful.** Real label text (ids 1200+1210) reconstructed across a header + bulleted line; bullet glyph dropped. |
| hamptonsun_reapply_swimming | **Faithful.** Real label text reconstructed across `•`-bulleted lines. |
| dermfree_use | **Faithful.** Real label text; source has `▇` glyph extraction artifacts. |
| noxivent_indication | **Faithful (fuzzy).** Hyphen/space normalization only. |
| cmc_eyedrops_storage | **UNCONFIRMED — earmarked for human audit / removal.** Quick source grep did not locate the storage sentence verbatim; degree-sign/range formatting may differ, or the span may be reworded. Resolve in the full human seed audit before submission; drop the example if not verifiable. |

Net: 254/255 cited spans are faithful to source (249 exact + 1 fuzzy + 4 benign reconstruction artifacts confirmed against PDFs). 1 span (`cmc_eyedrops_storage`) is unconfirmed and flagged for the human pass. The structural results (existence/in-context, non-overlap) do not depend on span text and are unaffected.
