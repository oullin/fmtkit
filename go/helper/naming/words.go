package naming

import "unicode"

// initialisms are Go's common initialisms, after golint's list. An
// initialism is one word whatever its case, as the generic split already
// gives; the list is what lets a plural one, IDs or URLs, stay one word too.
var initialisms = map[string]bool{
	"ACL":   true,
	"API":   true,
	"ASCII": true,
	"CPU":   true,
	"CSS":   true,
	"CSV":   true,
	"DNS":   true,
	"EOF":   true,
	"GUID":  true,
	"HTML":  true,
	"HTTP":  true,
	"HTTPS": true,
	"ID":    true,
	"IP":    true,
	"JSON":  true,
	"JWT":   true,
	"LHS":   true,
	"QPS":   true,
	"RAM":   true,
	"RHS":   true,
	"RPC":   true,
	"SLA":   true,
	"SMTP":  true,
	"SQL":   true,
	"SSH":   true,
	"TCP":   true,
	"TLS":   true,
	"TTL":   true,
	"UDP":   true,
	"UI":    true,
	"UID":   true,
	"URI":   true,
	"URL":   true,
	"UTF8":  true,
	"UUID":  true,
	"VM":    true,
	"XML":   true,
	"XMPP":  true,
	"XSRF":  true,
	"XSS":   true,
}

// Split returns the words of an identifier, the way fmtkit's TypeScript
// naming rules split names: any character that is not a letter or digit
// separates words, and so does a case change. An uppercase run ends before
// the word it leads, so HTTPServer is HTTP and Server, and userID is user and
// ID. A plural initialism, IDs or URLs, is one word.
func Split(name string) []string {
	runes := []rune(name)

	var words []string

	start := -1

	for i, r := range runes {
		if !unicode.IsLetter(r) && !unicode.IsDigit(r) {
			if start >= 0 {
				words = append(words, string(runes[start:i]))
				start = -1
			}

			continue
		}

		if start < 0 {
			start = i

			continue
		}

		prev := runes[i-1]
		lowerToUpper := (unicode.IsLower(prev) || unicode.IsDigit(prev)) && unicode.IsUpper(r)
		acronymEnd := unicode.IsUpper(prev) && unicode.IsUpper(r) &&
			i+1 < len(runes) && unicode.IsLower(runes[i+1]) && !pluralInitialism(runes, start, i)

		if lowerToUpper || acronymEnd {
			words = append(words, string(runes[start:i]))
			start = i
		}
	}

	if start >= 0 {
		words = append(words, string(runes[start:]))
	}

	return words
}

// pluralInitialism reports whether runes[start:at+1] is an initialism
// followed by a plural s that ends the word: the D in IDs, the L in URLs.
func pluralInitialism(runes []rune, start, at int) bool {
	if !initialisms[string(runes[start:at+1])] || runes[at+1] != 's' {
		return false
	}

	return at+2 == len(runes) || !unicode.IsLower(runes[at+2])
}
