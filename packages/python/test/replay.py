import json
import re
import sys

import gsx


FACTS = {
    "media": "media",
    "mediaCount": "media_count",
    "placement": "placement",
    "title": "title",
    "targetUrl": "target_url",
    "maxCharacters": "max_characters",
    "charactersReservedPerUrl": "characters_reserved_per_url",
    "captionLimit": "caption_limit",
    "descriptionLimit": "description_limit",
    "hashtagLimit": "hashtag_limit",
    "spoilerText": "spoiler_text",
    "overflow": "overflow",
    "entitiesAllowed": "entities_allowed",
}


def media_rule(case):
    if "mediaRule" in case:
        rule = {
            "mediaIdMaxBytes": gsx.MEDIA_ID_MAX_BYTES,
            "mediaGroupMin": gsx.MEDIA_GROUP_MIN,
            "mediaGroupMax": gsx.MEDIA_GROUP_MAX,
            "mediaIdPattern": gsx.MEDIA_ID_PATTERN,
        }
        return json.dumps(rule, separators=(",", ":"))
    valid = gsx.is_media_id(case["mediaId"])
    if (re.fullmatch(gsx.MEDIA_ID_PATTERN, case["mediaId"]) is not None) != valid:
        raise AssertionError("MEDIA_ID_PATTERN and is_media_id differ for " + repr(case["mediaId"]))
    return json.dumps(valid)


def main(out, files):
    names = [target.name for target in gsx.targets()]
    if "x-plain" not in names:
        raise AssertionError("targets() lists " + repr(names))
    if gsx.is_media_id("a" + chr(0xD800)) or gsx.is_media_id(chr(0xD83D) + chr(0xDE00)):
        raise AssertionError("is_media_id accepts surrogates")
    try:
        gsx.is_media_id(b"a")
        raise AssertionError("is_media_id accepts bytes")
    except TypeError:
        pass
    try:
        items = (gsx.MediaFact('x"/><img src="media:evil', "image"),)
        gsx.project_json("", "telegram-html-rich", gsx.Facts(items=items))
        raise AssertionError("project_json accepts an item ID outside the media ID rule")
    except gsx.InternalError as error:
        if str(error) != "item 0 id is not a media ID":
            raise
    lines = []
    for path in files:
        with open(path, encoding="utf-8") as handle:
            cases = [json.loads(line) for line in handle.read().split("\n")[1:] if line]
        for case in cases:
            if "mediaRule" in case or "mediaId" in case:
                lines.append(media_rule(case))
                continue
            markdown = case["markdown"]
            media = [gsx.Media(item["id"], item.get("cover")) for item in case["media"]]
            target = case.get("target")
            given = dict(case.get("facts", {}))
            listed = given.pop("items", None)
            items = None if listed is None else tuple(gsx.MediaFact(**item) for item in listed)
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
