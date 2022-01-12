package api

import "google.golang.org/api/sheets/v4"

const maxValue = 255.

func normalizeTone(tone float64) float64 { return tone / maxValue }

var ColorLightRed2 = &sheets.Color{
	Alpha: 1,
	Blue:  normalizeTone(153.),
	Green: normalizeTone(153.),
	Red:   normalizeTone(234.),
}
