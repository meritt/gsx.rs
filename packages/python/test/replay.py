import json
import sys

import gsx


FACTS = {
    "media": "media",
    "mediaCount": "media_count",
    "title": "title",
    "maxCharacters": "max_characters",
    "charactersReservedPerUrl": "characters_reserved_per_url",
    "captionLimit": "caption_limit",
    "entitiesAllowed": "entities_allowed",
}


def main(out, files):
    names = [target.name for target in gsx.targets()]
    if "x-plain" not in names:
        raise AssertionError("targets() lists " + repr(names))
    lines = []
    for path in files:
        with open(path, encoding="utf-8") as handle:
            cases = [json.loads(line) for line in handle.read().split("\n")[1:] if line]
        for case in cases:
            markdown = case["markdown"]
            media = [gsx.Media(item["id"], item.get("cover")) for item in case["media"]]
            target = case.get("target")
            given = dict(case.get("facts", {}))
            items = tuple(gsx.MediaFact(**item) for item in given.pop("items", []))
            facts = gsx.Facts(items=items, **{FACTS[key]: value for key, value in given.items()})
            raw = gsx.project_json(markdown, target, facts, media) if target else gsx.parse_json(markdown, media)
            wire = json.loads(raw)
            try:
                if target:
                    gsx.project(markdown, target, facts, media)
                    if "projection" not in wire:
                        raise AssertionError("project returned a projection for " + raw)
                else:
                    gsx.parse(markdown, media)
                    if "document" not in wire:
                        raise AssertionError("parse returned a document for " + raw)
            except gsx.ParseError as error:
                if error.code != wire.get("error", {}).get("code"):
                    raise
            lines.append(raw)
            if not target:
                raw = gsx.serialize_json(markdown, media)
                wire = json.loads(raw)
                try:
                    if gsx.serialize(markdown, media) != wire.get("serialized"):
                        raise AssertionError("serialize differs for " + raw)
                except gsx.ParseError as error:
                    if error.code != wire.get("error", {}).get("code"):
                        raise
                lines.append(raw)
    with open(out, "w", encoding="utf-8") as handle:
        handle.write("\n".join(lines) + "\n")


if __name__ == "__main__":
    if len(sys.argv) < 3:
        sys.exit("usage: replay.py <out.jsonl> <corpus.jsonl>...")
    main(sys.argv[1], sys.argv[2:])
