"""Conservative, opt-in cleanup. The original transcript is always retained."""

import re


def clean_text(text: str, mode="verbatim", remove_fillers=False) -> str:
    if mode not in ("verbatim", "clean", "paragraphs"):
        raise ValueError("Unknown writing mode.")
    if mode == "verbatim":
        return text
    # Do not alter identifiers, numbers, capitalization, punctuation or spelling.
    # Inline code, URLs, commands and all digit-bearing transcripts stay verbatim.
    if re.search(r"\d|https?://|`|[\\/_=]|--|\$", text):
        return text
    cleaned = re.sub(r"[ \t]+", " ", text).strip()
    if remove_fillers:
        cleaned = re.sub(r"(?i)(?<!\w)(?:um|uh)(?!\w)[, ]*", "", cleaned).strip()
    if mode == "paragraphs":
        cleaned = re.sub(r"(?<=[.!?।]) +(?=\S)", "\n\n", cleaned)
    return cleaned
