# Support matrix

Statuses: `supported`, `experimental`, `unsupported`, `not tested`.
Nothing has run on real Qualcomm hardware.

| Device / OS | Backend | SDK | Discovery | Compute | Evidence |
| --- | --- | --- | --- | --- | --- |
| Any, no Qualcomm SDK | HTP/GPU/CPU | none | supported (reports `Unavailable`) | unsupported | CI, mock + fake library |
| Android Snapdragon (e.g. 8 Gen 3) | HTP | not tested | not tested | not implemented | none |
| Android Snapdragon | GPU | not tested | not tested | not implemented | none |
| Windows ARM64 Snapdragon X | HTP | not tested | not tested | not implemented | none |
| Embedded QCS/SA | any | not tested | not tested | not implemented | none |
| Qualcomm CPU backend | CPU | not tested | not tested | not implemented | none |

A row changes only with a hardware report giving exact device model,
chipset, OS build, SDK version, driver/firmware and backend.
