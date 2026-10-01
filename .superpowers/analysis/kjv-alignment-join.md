# KJV → original-token alignment spike (CX-R3)

**Result: 707,678 of 790,892 atlas KJV tokens (89.4785%) have a complete bridge under the conservative rules below.** This is a measured lower bound, not a maximum attainable coverage estimate or a certification of the underlying alignment. Every other word remains explicitly unresolved. All source flags are retained; no English word, spelling or Scripture text was changed.

Base: atlas `678a0d2` (CONTRACT-2 close). The exact checked-in Kjv blob has SHA-256 `4fe493d1bcc4fe785cce51fbae1e40fbaa09f337956d4c8b10a8d946ab461467`, logical `6f413a55daedb14b489139d97814d214`, schema 18. Its decompressed SQLite is 262,615,040 bytes. The base manifest records root `b745ec41abf7f060966f66bfe45524fb`; this report uses the pinned commit/blob, rather than the queue’s later root summary. Text comes from that artifact’s `node.payload.TextUnit.renderings.kjv`, and word spans come from all 790,892 `kjv_token` rows. The mutable working tree and its raw KJV file were not used for the join.

## Coverage and unresolved work

Outcomes are mutually exclusive in priority order: English diff → source flag → supplied → absent source pointer → original-token bridge → aligned. Percentages use all 790,892 atlas tokens.

| Outcome | Words | Percent |
|---|---:|---:|
| Complete conservative bridge | 707,678 | 89.4785% |
| English edition mismatch or ambiguous diff | 1,594 | 0.2015% |
| ERROR/ERRO/SPLIT flag, or shares a flagged original token | 29,682 | 3.7530% |
| Supplied/italic in BibleForge, no original token | 26,977 | 3.4110% |
| OT original-token bridge withheld | 8,798 | 1.1124% |
| NT original-token bridge withheld | 16,163 | 2.0436% |

| Testament | Total words | Complete bridge | Percent |
|---|---:|---:|---:|
| OT | 610,324 | 549,757 | 90.0763% |
| NT | 180,568 | 157,921 | 87.4579% |

**Why the original-token bridge is withheld:** These are algorithm/source-interoperability buckets; they do not all mean the Greek TR text is missing. In particular, a difference between inflected-form and headword Strong’s codes can refuse an otherwise matching surface.

| Reason | Atlas words |
|---|---:|
| OT:lexicon_code_disagreement | 3,933 |
| OT:original_content_or_alignment_gap | 4,805 |
| OT:no_STEP_verse | 60 |
| NT:lexicon_code_disagreement | 10,426 |
| NT:TR_reordered_ambiguous | 5,216 |
| NT:original_content_or_alignment_gap | 521 |

- **English editions:** 453 verse token lists differ under the documented normalization, containing 12,212 atlas words. The 1,594 refused English tokens are a subset; uniquely matched words in a differing verse can still proceed. The earlier 452-verse research result is not reproduced exactly: this run uses the compiled token layer and the normalization below. Do not silently substitute 452 in the denominator. Psalm superscriptions are a concrete boundary difference (BibleForge verse 0 versus titles embedded in the atlas’s verse 1).
- **Source flags:** the downloaded English table has 18,197 notes containing `ERRO` (including the truncated `SPLIT ERRO`) and 19,514 containing `ERRO` or `SPLIT`. Testing only the literal `ERROR` misses thousands. This report conservatively quarantines all those annotations, including split annotations that may ultimately be valid. It also quarantines all English words sharing the implicated `orig_id`; a clean-looking neighbouring row must not inherit a suspect token mapping. The final 29,682 atlas-word bucket is therefore larger than the count of directly flagged rows.
- **Italics/supplied:** BibleForge marks 27,073 nonempty words implied. 26,995 matched atlas words carry this flag; 26,977 fall in the supplied outcome after earlier exclusions. This remains BibleForge’s supplied-word annotation, not a new authoritative italic markup layer for our edition.
- **TR work remains:** 268 K-bearing TAGNT records have edition/variant syntax this prototype refuses. In verses declaring a TR word-order displacement, only unique form+Strong’s signatures are accepted; repeated signatures remain unresolved. Qere/Ketiv reconciliation and explicit verse-boundary curation also remain work. A curated lexical-code crosswalk may recover many words, but no crosswalk is inferred from word number.

## Join rules

1. Read each atlas word by its stored zero-based ordinal and Unicode scalar start/end span from its own KJV rendering. Group by the artifact’s book/chapter/verse; BibleForge book numbers are explicitly converted from 1-based to the atlas’s 0-based indices.
2. Parse only the `bible_en` and `bible_original` tables. Ignore HTML/whole-verse and morphology tables also present in the dumps. Exclude empty English placeholder rows from the English diff; retain their source flags when quarantining original IDs.
3. Normalize English with NFC, case-folding, curly-apostrophe → straight-apostrophe and en-dash → hyphen, and trim exterior punctuation. Internal hyphens remain: `Beth–lehem` is not made equal to `Bethlehem`; `enquire` is not changed into `inquire`.
4. Compute a per-verse longest common subsequence of normalized English tokens. Keep a pair only when it occurs in **every** optimal alignment. Equal whole sequences are a checked special case. Ambiguous repeated words are refused, not assigned by their numerical positions. This establishes atlas ordinal → BibleForge English row ID.
5. Follow the source’s explicit `orig_id`, then compare that original token against the STEPBible inventory. Hebrew normalization removes vowel/cantillation marks and separators, preserving consonants; STEPBible’s backslash-attached punctuation/parashah markers are excluded. Use its `L` rows, preserving the full reference including alternate Hebrew numbering. Qere-only alternatives are not synthesized.
6. For Greek, use K-bearing TAGNT rows, the actual TR edition membership, its explicit TR significant variants and TR spelling variants. Apply square-bracket KJV verse overrides; preserve the full source reference. Never call the default NA word the TR word when the columns specify a different one. Strip diacritics/case/punctuation for form comparison. TR displacement syntax is recognized; it is not guessed into an ordering.
7. Require both a matching normalized original form and agreement with one of the source row’s numeric Strong’s codes. Strong’s alone never identifies a token. Use the same all-optimal-alignment rule for ordered verse sequences, with an additional independently unique form+Strong’s pair rule. In TR-reordered verses use only that unique signature rule. Confirm the resulting original-token bridge is injective; this run found zero many-to-one collisions.
8. Quarantine source flags, apply the supplied-word rule, and emit one audit row per atlas token with both source IDs, original spelling, complete STEPBible reference, status and refusal reason. Successful rows retain the original Hebrew/Greek spelling variants used in the match.

**Validation:** SQL row arities and source table counts checked; 814,695 English rows, 790,850 nonempty English words, 446,232 original-language rows. BibleForge has 763,778 nonempty English rows with a nonzero `orig_id`. The ambiguity matcher agrees with exhaustive enumeration of all optimal alignments for 961 pairs of short repeated-token sequences. The final audit has exactly 790,892 unique `(reference, ordinal)` keys, its outcome counts sum to the denominator, and every successful row has both original IDs and no quarantine flag. A shared-original collision check is zero. These checks validate this spike’s accounting and conservative matching, not the historical accuracy of every source annotation.

## Twenty worked examples

Ordinals below are zero-based atlas `kjv_token.ord`. English-row and original-row IDs are BibleForge IDs, not Strong’s numbers. A source-flagged row can have a mechanical candidate but is still withheld. Each successful row passed both English comparison and original-form/code checks.

| Atlas token | BibleForge English → original | Original spelling | STEPBible token / decision |
|---|---|---|---|
| Gen.1.1 #2 **beginning** | 3 → 1 | בְּרֵאשִׁ֖ית | Gen.1.1#01=L |
| Gen.1.1 #3 **God** | 4 → 3 | אֱלֹהִ֑ים | Gen.1.1#03=L |
| Gen.1.2 #10 **was** | 22 → 0 | — | supplied; withheld: supplied |
| Gen.1.9 #21 **and** | 182 → 102 | וַֽיְהִי | Gen.1.9#12=L; withheld: flagged_source |
| Gen.24.57 #9 **enquire** | unresolved | — | english_diff_or_ambiguity; withheld: english_diff_or_ambiguity |
| Gen.32.28 #10 **Jacob** | 24186 → 12766 | יַֽעֲקֹב֙ | Gen.32.28(32.29)#03=L |
| Gen.32.28 #12 **Israel** | 24189 → 12772 | יִשְׂרָאֵ֑ל | Gen.32.28(32.29)#09=L |
| Gen.35.10 #19 **Israel** | 25914 → 13684 | יִשְׂרָאֵל֙ | Gen.35.10#13=L |
| Gen.35.10 #29 **Israel** | 25925 → 13690 | יִשְׂרָאֵֽל׃ | Gen.35.10#19=L |
| Gen.35.19 #13 **Beth–lehem** | unresolved | — | english_diff_or_ambiguity; withheld: english_diff_or_ambiguity |
| Num.20.21 #1 **Edom** | 117564 → 58795 | אֱד֗וֹם | Num.20.21#02=L |
| Num.20.21 #5 **Israel** | 117569 → 58798 | יִשְׂרָאֵ֔ל | Num.20.21#05=L |
| Num.20.21 #11 **Israel** | 117575 → 58802 | יִשְׂרָאֵ֖ל | Num.20.21#09=L |
| Jos.15.1 #21 **Edom** | 171706 → 85818 | אֱד֧וֹם | Jos.15.1#09=L |
| Mat.1.1 #11 **David** | 621730 → 305480 | Δαβίδ, | Mat.1.1#06=NKO; TR-spelling Δαβὶδ |
| Mat.1.10 #7 **Amon** | 621888 → 305630 | Ἀμών· | Mat.1.10#10=N(k)O; TR-variant Ἀμών |
| Jhn.1.1 #5 **Word** | 691005 → 355852 | λόγος | Jhn.1.1#05=NKO |
| Jhn.1.1 #3 **was** | 691003 → 355850 | ἦν | lexicon_code_disagreement; withheld: NT_TR_content_or_versification_gap |
| Act.1.5 #10 **baptized** | 711548 → 371868 | βαπτισθήσεσθε | Act.1.5#10=NKO |
| 1Jn.5.7 #12 **Word** | 800192 → 435035 | λόγος, | 1Jn.5.7#12=K |

Genesis 32:28 **Israel** is an explicit path: atlas ordinal 12 → BibleForge English row 24189 → original row 12772 → `Gen.32.28(32.29)#09=L`. The two occurrences of Israel in Genesis 35:10 reach original rows 13684 and 13690 and STEP tokens #13 and #19 respectively. They are not collapsed to H3478. Matthew 1:10 Amon uses TAGNT’s TR variant, not its default Amos form. John 1:1’s repeated “was” illustrates a lexical-code refusal; its absence from the accepted set is not evidence that TR lacks that word.

## Sources and licensing

- [BibleForgeDB](https://github.com/bibleforge/BibleForgeDB/tree/e2b458cd5a834fd5788c85e304a9f5561e9f696b), pinned `e2b458cd5a834fd5788c85e304a9f5561e9f696b`. Used only `bible_en_all.sql.gz`, `bible_original.sql.gz`, `readme.txt` and `licenses/cc0.txt`; data declared public domain/CC0. No Chinese data or repository scripts were used.
- [STEPBible Data](https://github.com/STEPBible/STEPBible-Data/tree/b99716b0cddb648ddb95cc786a197180f2f97d48), pinned `b99716b0cddb648ddb95cc786a197180f2f97d48`. Used the six TAHOT/TAGNT files under `Translators Amalgamated OT+NT/` and README.md. Attribution: **STEP Bible / Tyndale House Cambridge**, [STEPBible.org](https://www.STEPBible.org), [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/). This spike changes the representation for comparison and records accepted/refused matches; it does not amend their source texts.
- Atlas `678a0d2:LICENSES.md` declares the KJV text public domain. The literal license-table entry is: `| KJV text ([scrollmapper/bible_databases](https://github.com/scrollmapper/bible_databases)) | Public domain | Redistributed — compiled into verses-kjv.json and canon.json |` (code formatting removed only). Only the existing compiled KJV rendering and token offsets were read; no other translation rendering was used.

### BibleForge source license declaration (readme.txt, verbatim)

```text
BibleForge DB

Most of these files are compressed SQL database tables used in the BibleForge project (http://bibleforge.com).

Licenses:

All code is copyrighted under the MIT License (MIT), which can be found in licenses/mit.txt.

The texts of the King James Bible, the original Greek and Hebrew, the Greek and Hebrew Lexicons,
and all other related data, such as the Strong's numbers and grammatical information, are in the public domain.

The Chinese King James Version (中文英皇欽定本) (bible_zh_s_all.sql.gz and bible_zh_t_all.sql.gz) is distributed
under the Creative Commons Attribution-ShareAlike 3.0 Unported (CC BY-SA 3.0).
The CKJV was created by the team from http://ckjv.asia/.

To the extent possible under law, the author(s) have dedicated all copyright and related and
neighboring rights to any and all other data in this repository not mentioned above
to the public domain worldwide. Details can be found in licenses/cc0.txt.

This software is distributed without any warranty. 

For more info about the licenses, please see the following links:
MIT License (MIT): http://www.opensource.org/licenses/MIT
Public Domain Mark: https://creativecommons.org/publicdomain/mark/1.0/
Public Domain Dedication (CC0 1.0): https://creativecommons.org/publicdomain/zero/1.0/
CC BY-SA 3.0: https://creativecommons.org/licenses/by-sa/3.0/deed.en
```

### STEPBible license declarations

Each of the following six source files contains the shared license block reproduced below. The source-specific first-line declaration is also reproduced. The accompanying README calls the data Creative Commons Attribution 4.0 International. The retained provenance must include the source reference, source revision, attribution and this prototype’s normalization/variant decisions.

- `TAGNT Act-Rev - Translators Amalgamated Greek NT - STEPBible.org CC-BY.txt`: TAGNT Mat-Jhn - Translators Amalgamated Greek NT -  STEPBible.org CC BY 4.0
- `TAGNT Mat-Jhn - Translators Amalgamated Greek NT - STEPBible.org CC-BY.txt`: TAGNT Mat-Jhn - Translators Amalgamated Greek NT -  STEPBible.org CC BY 4.0
- `TAHOT Gen-Deu - Translators Amalgamated Hebrew OT - STEPBible.org CC BY.txt`: TAHOT Gen-Deu - Translators Amalgamated Hebrew OT - STEPBible.org CC BY.txt
- `TAHOT Isa-Mal - Translators Amalgamated Hebrew OT - STEPBible.org CC BY.txt`: TAHOT Gen-Deu - Translators Amalgamated Hebrew OT - STEPBible.org CC BY.txt
- `TAHOT Job-Sng - Translators Amalgamated Hebrew OT - STEPBible.org CC BY.txt`: TAHOT Gen-Deu - Translators Amalgamated Hebrew OT - STEPBible.org CC BY.txt
- `TAHOT Jos-Est - Translators Amalgamated Hebrew OT - STEPBible.org CC BY.txt`: TAHOT Gen-Deu - Translators Amalgamated Hebrew OT - STEPBible.org CC BY.txt

```text
Data created by www.STEPBible.org based on work at Tyndale House Cambridge (CC BY 4.0)
==============================================================
This licence allows you to:
* Include any part of this data in software or publications without requesting permission
* Download the data and reformat it for your application, without changing the data
* Send any proposed corrections to STEPBibleATGmail.com. to be verified
(You MAY make changes yourself, but you should include a note of changes that can be viewed by those who use your new data)
* Refer others to github.com/STEPBible as the source of the data. Please do not redistribute it yourself.
(Updates or corrections are easier to implement when the data is distributed from a single source)
* We'd love to hear about your project when you make it available. Email us at STEPBibleATGmail.com..
```

### BibleForge CC0 dedication (licenses/cc0.txt, verbatim)

```text
Creative Commons Legal Code

CC0 1.0 Universal

    CREATIVE COMMONS CORPORATION IS NOT A LAW FIRM AND DOES NOT PROVIDE
    LEGAL SERVICES. DISTRIBUTION OF THIS DOCUMENT DOES NOT CREATE AN
    ATTORNEY-CLIENT RELATIONSHIP. CREATIVE COMMONS PROVIDES THIS
    INFORMATION ON AN "AS-IS" BASIS. CREATIVE COMMONS MAKES NO WARRANTIES
    REGARDING THE USE OF THIS DOCUMENT OR THE INFORMATION OR WORKS
    PROVIDED HEREUNDER, AND DISCLAIMS LIABILITY FOR DAMAGES RESULTING FROM
    THE USE OF THIS DOCUMENT OR THE INFORMATION OR WORKS PROVIDED
    HEREUNDER.

Statement of Purpose

The laws of most jurisdictions throughout the world automatically confer
exclusive Copyright and Related Rights (defined below) upon the creator
and subsequent owner(s) (each and all, an "owner") of an original work of
authorship and/or a database (each, a "Work").

Certain owners wish to permanently relinquish those rights to a Work for
the purpose of contributing to a commons of creative, cultural and
scientific works ("Commons") that the public can reliably and without fear
of later claims of infringement build upon, modify, incorporate in other
works, reuse and redistribute as freely as possible in any form whatsoever
and for any purposes, including without limitation commercial purposes.
These owners may contribute to the Commons to promote the ideal of a free
culture and the further production of creative, cultural and scientific
works, or to gain reputation or greater distribution for their Work in
part through the use and efforts of others.

For these and/or other purposes and motivations, and without any
expectation of additional consideration or compensation, the person
associating CC0 with a Work (the "Affirmer"), to the extent that he or she
is an owner of Copyright and Related Rights in the Work, voluntarily
elects to apply CC0 to the Work and publicly distribute the Work under its
terms, with knowledge of his or her Copyright and Related Rights in the
Work and the meaning and intended legal effect of CC0 on those rights.

1. Copyright and Related Rights. A Work made available under CC0 may be
protected by copyright and related or neighboring rights ("Copyright and
Related Rights"). Copyright and Related Rights include, but are not
limited to, the following:

  i. the right to reproduce, adapt, distribute, perform, display,
     communicate, and translate a Work;
 ii. moral rights retained by the original author(s) and/or performer(s);
iii. publicity and privacy rights pertaining to a person's image or
     likeness depicted in a Work;
 iv. rights protecting against unfair competition in regards to a Work,
     subject to the limitations in paragraph 4(a), below;
  v. rights protecting the extraction, dissemination, use and reuse of data
     in a Work;
 vi. database rights (such as those arising under Directive 96/9/EC of the
     European Parliament and of the Council of 11 March 1996 on the legal
     protection of databases, and under any national implementation
     thereof, including any amended or successor version of such
     directive); and
vii. other similar, equivalent or corresponding rights throughout the
     world based on applicable law or treaty, and any national
     implementations thereof.

2. Waiver. To the greatest extent permitted by, but not in contravention
of, applicable law, Affirmer hereby overtly, fully, permanently,
irrevocably and unconditionally waives, abandons, and surrenders all of
Affirmer's Copyright and Related Rights and associated claims and causes
of action, whether now known or unknown (including existing as well as
future claims and causes of action), in the Work (i) in all territories
worldwide, (ii) for the maximum duration provided by applicable law or
treaty (including future time extensions), (iii) in any current or future
medium and for any number of copies, and (iv) for any purpose whatsoever,
including without limitation commercial, advertising or promotional
purposes (the "Waiver"). Affirmer makes the Waiver for the benefit of each
member of the public at large and to the detriment of Affirmer's heirs and
successors, fully intending that such Waiver shall not be subject to
revocation, rescission, cancellation, termination, or any other legal or
equitable action to disrupt the quiet enjoyment of the Work by the public
as contemplated by Affirmer's express Statement of Purpose.

3. Public License Fallback. Should any part of the Waiver for any reason
be judged legally invalid or ineffective under applicable law, then the
Waiver shall be preserved to the maximum extent permitted taking into
account Affirmer's express Statement of Purpose. In addition, to the
extent the Waiver is so judged Affirmer hereby grants to each affected
person a royalty-free, non transferable, non sublicensable, non exclusive,
irrevocable and unconditional license to exercise Affirmer's Copyright and
Related Rights in the Work (i) in all territories worldwide, (ii) for the
maximum duration provided by applicable law or treaty (including future
time extensions), (iii) in any current or future medium and for any number
of copies, and (iv) for any purpose whatsoever, including without
limitation commercial, advertising or promotional purposes (the
"License"). The License shall be deemed effective as of the date CC0 was
applied by Affirmer to the Work. Should any part of the License for any
reason be judged legally invalid or ineffective under applicable law, such
partial invalidity or ineffectiveness shall not invalidate the remainder
of the License, and in such case Affirmer hereby affirms that he or she
will not (i) exercise any of his or her remaining Copyright and Related
Rights in the Work or (ii) assert any associated claims and causes of
action with respect to the Work, in either case contrary to Affirmer's
express Statement of Purpose.

4. Limitations and Disclaimers.

 a. No trademark or patent rights held by Affirmer are waived, abandoned,
    surrendered, licensed or otherwise affected by this document.
 b. Affirmer offers the Work as-is and makes no representations or
    warranties of any kind concerning the Work, express, implied,
    statutory or otherwise, including without limitation warranties of
    title, merchantability, fitness for a particular purpose, non
    infringement, or the absence of latent or other defects, accuracy, or
    the present or absence of errors, whether or not discoverable, all to
    the greatest extent permissible under applicable law.
 c. Affirmer disclaims responsibility for clearing rights of other persons
    that may apply to the Work or any use thereof, including without
    limitation any person's Copyright and Related Rights in the Work.
    Further, Affirmer disclaims responsibility for obtaining any necessary
    consents, permissions or other rights required for any use of the
    Work.
 d. Affirmer understands and acknowledges that Creative Commons is not a
    party to this document and has no duty or obligation with respect to
    this CC0 or use of the Work.
```

## Reproduction and handoff

Scratch directory: `/home/donovan/w/CX-R3-scratch`. `join.py` reads pinned downloads plus `kjv-close.sqlite` unpacked from the exact git blob above, and emits `summary.json`, `differing-verses.json` and `word-audit.jsonl`. `check_matching.py` exhaustively tests the ambiguity rule. `report.py` reconciles all audit rows against the summary and emits this report. Nothing from scratch is a product input; only this report is committed.

Run `python3 join.py`, `python3 check_matching.py`, then `python3 report.py` in the scratch directory. Acquisition: clone BibleForgeDB at the pinned SHA; download only the six named STEP files and README from raw.githubusercontent.com at the STEP SHA; decompress the pinned Kjv blob with libzstd. Cache `.pickle` files are derived scratch and can be regenerated; no upstream files are modified.

For A-NAMES: preserve `(translation, verse identity, token ordinal) → source English ID → source original ID → full STEP reference`, plus refusal reason and all source flags. Do not substitute a Strong’s number or a bare word number for a token identity. The next coverage work is an explicit inflected/headword code crosswalk, TR reorder/variant parsing, superscription/verse-boundary maps, and review of source flags; this spike does not authorize production ingestion or resolve theological titles.

### Input and algorithm SHA-256 manifest

```text
a1031cd9a172c439ec648ab57e30292857a57ce67cdf6ea687a9ca12a6ea1ed6  BibleForgeDB/bible_en_all.sql.gz
4c04e76476ff1a77b2b1f9fbf06ef32abb58a20ddd935e9a2fe43e9dc1a4ab8b  BibleForgeDB/bible_original.sql.gz
a94c9efea43d2adb6ac2b77f9dfa21546ade7ea09e29e730ecbf7b1ac8241fa7  BibleForgeDB/readme.txt
a2010f343487d3f7618affe54f789f5487602331c0a8d03f49e9a7c547cf0499  BibleForgeDB/licenses/cc0.txt
261d5157c0ffeadeedad3f734db945a4c3642e3e4ce5fa28002985b6c52437b1  STEPBible/README.md
524e32375361e6d3fa2f7ef00b87605fdc4317a762f395651a05fdc31ad031b7  STEPBible/TAGNT Act-Rev - Translators Amalgamated Greek NT - STEPBible.org CC-BY.txt
ab8eaaeb68e17a1dcfa34e1e9350358f22f03bc2a97244d848750ad81044bc8e  STEPBible/TAGNT Mat-Jhn - Translators Amalgamated Greek NT - STEPBible.org CC-BY.txt
e9b8546ee48fe0bfc57c3b70f5f40e98d96580e803526d19026224e31753368b  STEPBible/TAHOT Gen-Deu - Translators Amalgamated Hebrew OT - STEPBible.org CC BY.txt
f3ded203d2a74d6368932c97ae550d1d0754b271af491dc0dedf36fe3ba0bcc5  STEPBible/TAHOT Isa-Mal - Translators Amalgamated Hebrew OT - STEPBible.org CC BY.txt
84e118a97e5725e3847cdfdd593873513021c790c63cc91a0d41fca2b5db2ed5  STEPBible/TAHOT Job-Sng - Translators Amalgamated Hebrew OT - STEPBible.org CC BY.txt
195fee1dc3653bab33701f170734eb894ed647c10cd08cc61749375fe8b73775  STEPBible/TAHOT Jos-Est - Translators Amalgamated Hebrew OT - STEPBible.org CC BY.txt
f18d8c0eaa899ec29623d8973210efe7064b6890157b625ecd294c3ef7742189  kjv-close.sqlite
414463103e3794f2dbc6f76d7090049a86a9c6564f20c7ca46f56790dad0e4c8  join.py
b0ace3785b541249ee0975d3eafb761717283b29e82ed6d586fc0ee7a34b74ed  check_matching.py
28fc57417e3841c33e7a54d48ce1066f3caa17c87afb00e57cd07a5c1b6e1818  summary.json
```

### Exact spike algorithm

```python
import collections,gzip,json,pathlib,pickle,re,sqlite3,unicodedata
R=pathlib.Path(__file__).parent
BOOKS='Gen Exo Lev Num Deu Jos Jdg Rut 1Sa 2Sa 1Ki 2Ki 1Ch 2Ch Ezr Neh Est Job Psa Pro Ecc Sng Isa Jer Lam Ezk Dan Hos Jol Amo Oba Jon Mic Nam Hab Zep Hag Zec Mal Mat Mrk Luk Jhn Act Rom 1Co 2Co Gal Eph Php Col 1Th 2Th 1Ti 2Ti Tit Phm Heb Jas 1Pe 2Pe 1Jn 2Jn 3Jn Jud Rev'.split()
BOOK_IDS={b:i+1 for i,b in enumerate(BOOKS)}
ROW=re.compile(r"\((?:'(?:\\.|[^'\\])*'|[^'()])*\)")
VALUE=re.compile(r"'(?:\\.|[^'\\])*'|NULL|-?\d+")
def sql(name):
 cache=R/(name+'.pickle')
 if cache.exists(): return pickle.loads(cache.read_bytes())
 rows=[]
 with gzip.open(R/'BibleForgeDB'/(name+'.sql.gz'),'rt') as f:
  for line in f:
   table='bible_en' if name=='bible_en_all' else name
   if not line.startswith('INSERT INTO `'+table+'` VALUES '): continue
   for m in ROW.finditer(line[line.index('VALUES ')+7:]):
    vals=[]
    for raw in VALUE.findall(m.group()):
     if raw.startswith("'"): vals.append(re.sub(r'\\(.)',lambda x:{'n':'\n','r':'\r','t':'\t','0':'\0'}.get(x[1],x[1]),raw[1:-1]))
     else: vals.append(None if raw=='NULL' else int(raw))
    rows.append(vals)
 cache.write_bytes(pickle.dumps(rows)); return rows

def english(s):
 s=unicodedata.normalize('NFC',s).replace('’',"'").replace('–','-').casefold()
 while s and not s[0].isalnum(): s=s[1:]
 while s and not s[-1].isalnum(): s=s[:-1]
 return s

def original(s):
 return ''.join(c for c in unicodedata.normalize('NFD',s).casefold() if unicodedata.category(c).startswith('L')).replace('ς','σ')

def certain(a,b):
 if a==b: return dict(enumerate(range(len(a))))
 n,m=len(a),len(b); f=[[0]*(m+1) for _ in range(n+1)]; z=[[0]*(m+1) for _ in range(n+1)]
 for i in range(n):
  for j in range(m): f[i+1][j+1]=f[i][j]+1 if a[i]==b[j] else max(f[i][j+1],f[i+1][j])
 for i in range(n-1,-1,-1):
  for j in range(m-1,-1,-1): z[i][j]=z[i+1][j+1]+1 if a[i]==b[j] else max(z[i+1][j],z[i][j+1])
 length=f[n][m]; result={}
 for i in range(n):
  candidates=[j for j in range(m) if a[i]==b[j] and f[i][j]+1+z[i+1][j+1]==length]
  if len(candidates)==1 and max(f[i][j]+z[i+1][j] for j in range(m+1))<length: result[i]=candidates[0]
 return result

def read_step():
 groups=collections.defaultdict(list); counts=collections.Counter(); unresolved=[]
 for p in sorted((R/'STEPBible').glob('T*.txt')):
  for line in p.read_text().splitlines():
   cols=line.split('\t'); m=re.match(r'([123]?[A-Za-z]+)\.(\d+)\.(\d+)(.*?)#(\d+[^=]*)=(\S+)',cols[0])
   if not m: continue
   book,ch,vs,extra,wordno,flags=m.groups()
   if book not in BOOK_IDS: counts['unknown_book:'+book]+=1; continue
   bk=BOOK_IDS[book]; ch=int(ch); vs=int(vs); order=(ch,vs,int(re.match(r'\d+',wordno)[0]),wordno)
   kjv=re.search(r'\[(\d+)\.(\d+)\]',extra)
   if kjv: ch,vs=map(int,kjv.groups()); counts['explicit_kjv_ref']+=1
   if bk<40:
    if flags!='L': counts['hebrew_non_L']+=1; continue
    word=cols[1].split('\\')[0]; evidence='L'; strongs=cols[4]
   else:
    if 'k' not in flags.casefold(): continue
    word=None; evidence='TR'; strongs=cols[3]
    if re.search(r'(?:^|\+)TR(?:[«»][0-9]+)?(?:\+|$)',cols[5]):
     word=cols[1].split(' (')[0]
     for variant in cols[7].split(';'):
      if ':' in variant:
       editions,form=variant.split(':',1)
       if 'TR' in editions.strip().strip('+').split('+'): word=form.strip(); evidence='TR-spelling'; counts[evidence]+=1
    else:
     for variant in cols[6].split(';'):
      if ' in: ' in variant:
       form,editions=variant.rsplit(' in: ',1)
       if re.search(r'(?:^|\+)TR(?:[«»][0-9]+)?(?:\+|$)',editions.strip()):
        word=form.split(' (')[0]; strongs=' '.join(re.findall(r'G[0-9]+',form)); evidence='TR-variant'; counts[evidence]+=1
    if word is None:
     counts['unparsed_TR_variant']+=1; unresolved.append(cols[0]); continue
   groups[(bk,ch,vs)].append({'id':cols[0],'word':word,'norm':original(word),'strongs':strongs,'evidence':evidence,'order':order,'reordered':bk>=40 and bool(re.search(r'TR[«»]',cols[5]+' '+cols[6]))})
 for rows in groups.values(): rows.sort(key=lambda row:row['order'])
 return groups,counts,unresolved

def flagged(note):
 return 'ERRO' in note.upper() or 'SPLIT' in note.upper()

def main():
 assert certain(['a','x','a'],['a','a'])=={0:0,2:1}
 assert certain(['a','a'],['a'])=={}
 assert certain(['a','b'],['a','x','b'])=={0:0,1:2}
 print('parser start',flush=True)
 engrows=sql('bible_en_all'); origrows=sql('bible_original')
 assert all(len(row)==15 for row in engrows); assert all(len(row)==13 for row in origrows)
 eng=collections.defaultdict(list); orig=collections.defaultdict(list); original_ids={}
 for row in engrows:
  if row[5]: eng[tuple(row[2:5])].append(row)
 for rows in eng.values(): rows.sort(key=lambda row:row[11])
 for row in origrows: orig[tuple(row[2:5])].append(row); original_ids[row[0]]=row
 for rows in orig.values(): rows.sort(key=lambda row:(row[9],row[0]))
 flagged_ids={row[12] for row in engrows if row[12] and flagged(row[14])} | {row[0] for row in origrows if flagged(row[12])}
 print('sql loaded',len(engrows),len(origrows),flush=True)
 steps,step_counts,unparsed=read_step(); bridges={}; bridge_reasons={}; bridge_stats=collections.Counter()
 for key,rows in orig.items():
  target=steps.get(key,[]); a=[original(row[5]) for row in rows]; b=[row['norm'] for row in target]
  reordered=any(t['reordered'] for t in target)
  for row in rows: bridge_reasons[row[0]]='no_STEP_verse' if not target else ('TR_reordered_ambiguous' if reordered else 'original_content_or_alignment_gap')
  matched={} if reordered else certain(a,b)
  for i,row in enumerate(rows):
   candidates=[j for j,t in enumerate(target) if a[i]==b[j] and row[7] in {int(x) for x in re.findall(r'[HG]([0-9]{4})',t['strongs'])}]
   if len(candidates)==1 and sum(x[7]==row[7] and original(x[5])==a[i] for x in rows)==1: matched[i]=candidates[0]
  for i,j in matched.items():
   if not a[i]: continue
   if rows[i][7] not in {int(x) for x in re.findall(r'[HG]([0-9]{4})',target[j]['strongs'])}:
    bridge_stats['strongs_disagreement']+=1; bridge_reasons[rows[i][0]]='lexicon_code_disagreement'; continue
   bridges[rows[i][0]]=target[j]
   bridge_stats['OT' if key[0]<40 else 'NT']+=1
 uses=collections.Counter(row['id'] for row in bridges.values())
 collisions=[oid for oid,row in bridges.items() if uses[row['id']]>1]
 for oid in collisions:
  bridge_reasons[oid]='ambiguous_many_to_one'; del bridges[oid]
 bridge_stats['ambiguous_many_to_one']=len(collisions)
 assert len({row['id'] for row in bridges.values()})==len(bridges)
 print('original joins',dict(bridge_stats),flush=True)
 db=sqlite3.connect('file:'+str(R/'kjv-close.sqlite')+'?mode=ro',uri=True)
 texts={}
 for book,ch,vs,payload in db.execute('select v.book,v.chapter,v.verse,n.payload from verse v join node n on n.id=v.node_id'):
  texts[(book+1,ch,vs)]=json.loads(payload)['payload']['TextUnit']['renderings']['kjv']
 ours=collections.defaultdict(list)
 for book,ch,vs,ordinal,start,end in db.execute('select * from kjv_token'):
  key=(book+1,ch,vs); ours[key].append((ordinal,texts[key][start:end]))
 counts=collections.Counter(); gap_reasons=collections.Counter(); overlaps=collections.Counter(); differing=[]; examples=[]; bytestament=collections.defaultdict(collections.Counter)
 audit=R/'word-audit.jsonl'
 with audit.open('w') as output:
  for key,rows in ours.items():
   theirs=eng.get(key,[]); a=[english(row[1]) for row in rows]; b=[english(row[5]) for row in theirs]; mapping=certain(a,b)
   if a!=b: differing.append({'ref':key,'ours':a,'bibleforge':b})
   for i,(ordinal,word) in enumerate(rows):
    e=theirs[mapping[i]] if i in mapping else None
    o=original_ids.get(e[12]) if e else None; step=bridges.get(e[12]) if e else None
    flags=[]
    if a!=b: flags.append('differing_verse')
    if e and e[10]: flags.append('supplied')
    if e and flagged(e[14]): flags.append('english_source_flag')
    if e and e[12] in flagged_ids: flags.append('shared_original_source_flag')
    if o and flagged(o[12]): flags.append('original_source_flag')
    if e is None: outcome='english_diff_or_ambiguity'
    elif any(flag.endswith('source_flag') for flag in flags): outcome='flagged_source'
    elif e[10]: outcome='supplied'
    elif not e[12]: outcome='no_original_id'
    elif o is None: outcome='missing_original_row'
    elif step is None: outcome='OT_content_or_versification_gap' if key[0]<40 else 'NT_TR_content_or_versification_gap'
    else: outcome='aligned'
    if outcome.endswith('_gap'): gap_reasons[('OT:' if key[0]<40 else 'NT:')+bridge_reasons.get(e[12],'missing_source')]+=1
    counts[outcome]+=1; bytestament['OT' if key[0]<40 else 'NT'][outcome]+=1
    for flag in flags: overlaps[flag]+=1
    result={'ref':f'{BOOKS[key[0]-1]}.{key[1]}.{key[2]}','word':ordinal,'text':word,'outcome':outcome,'flags':flags,'bf_english':e[0] if e else None,'bf_original':e[12] if e else None,'bf_original_ref':o[2:5] if o else None,'original':o[5] if o else None,'step':step['id'] if step else None,'step_form':step['word'] if step else None,'step_evidence':step['evidence'] if step else None,'gap_reason':bridge_reasons.get(e[12]) if e and not step else None}
    output.write(json.dumps(result,ensure_ascii=False)+'\n')
    if len(examples)<30 and outcome!='aligned': examples.append(result)
 summary={'gap_reasons':gap_reasons,'counts':counts,'by_testament':bytestament,'overlapping_flags':overlaps,'differing_verses':len(differing),'step_parser':step_counts,'unparsed_TR_examples':unparsed[:20],'bf_rows':len(engrows),'bf_nonempty_words':sum(bool(x[5]) for x in engrows),'bf_originals':len(origrows),'original_bridges':bridge_stats}
 (R/'summary.json').write_text(json.dumps(summary,ensure_ascii=False,indent=2)); (R/'differing-verses.json').write_text(json.dumps(differing,ensure_ascii=False))
 print(json.dumps(summary,indent=2),flush=True)
if __name__=='__main__': main()
```
