# tinygrib2

## GRIB2 Sections

https://codes.ecmwf.int/grib/format/grib2/overview/

```mermaid
flowchart TB
    0[0: Indicator]
    1[1: Identification]
    2[2: Local Use]
    3[3: Grid Definition]
    4[4: Product Definition]
    5[5: Data Representation]
    6[6: Bit-Map]
    7[7: Data]
    8[8: End]
    0 --> 1
    1 --> 2
    1 --> 3
    2 --> 3
    3 --> 4
    4 --> 5
    5 --> 6
    6 --> 7
    7 --> 2
    7 --> 3
    7 --> 4
    7 --> 8
```

## Code tables and templates

- GRIB2: https://github.com/wmo-im/grib2
  - ECMWF: https://codes.ecmwf.int/grib/format/grib2/
- CCT (Common Code Tables): https://github.com/wmo-im/CCT
