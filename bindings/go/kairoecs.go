package kairoecs

const Version = "0.1.0"

func SelfCheck() map[string]string {
	return map[string]string{
		"package": "kairoecs",
		"version": Version,
		"status":  "ok",
	}
}
