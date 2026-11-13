Launch at ::time[noon]{datetime="2026-11-08T12:00:00Z"} and again ::time[later]{unix="1794139200"}.

- ::time[t]{datetime="2026-11-08T12:00:00Z" format="time"}
- ::time[ts]{unix="1794139200" format="time-seconds"}
- ::time[ds]{datetime="2026-11-08T12:00:00+03:00" format="date-short"}
- ::time[d]{format="date" datetime="2026-11-08"}
- ::time[dts]{datetime="2026-11-08T12:00Z" format="datetime-short"}
- ::time[dt]{unix="0" format="datetime"}
- ::time[f]{unix="2147483647" format="full"}
- ::time[r]{unix="-1" format="relative"}

Refused: ::time[x]{datetime="2026-11-08T12:00:00Z" format="Relative"} ::time[y]{format="time"} ::time[z]{unix="1" format=""} ::time[w]{unix="1" format="time" format="date"} ::time[**bold** label]{unix="1"}

> Quoted ::time[in a quote]{datetime="2026-11-08T12:00:00Z" format="relative"} with *emphasis*
