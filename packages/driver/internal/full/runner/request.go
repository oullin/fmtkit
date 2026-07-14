package runner

import "strings"

type languageFamily uint8

type request struct {
	command string
	args    []string
	family  languageFamily
}

const (
	allLanguages languageFamily = iota
	goLanguage
	tsLanguage
)

func (r Runner) validateRequest(args []string) (request, int) {
	if len(args) == 0 {
		return request{command: "format", family: allLanguages}, 0
	}

	command := strings.TrimSpace(args[0])

	switch command {
	case "format":
		return r.parseFormatRequest(args[1:])
	case "format-all":
		if len(args) != 1 {
			r.printUsage()

			return request{}, 2
		}

		return request{command: command, args: []string{"."}, family: allLanguages}, 0
	case "go", "ts", "check", "version", "--version", "-version", "help", "--help", "-h":
		if command == "ts" {
			for _, path := range args[1:] {
				if strings.TrimSpace(path) == "" {
					writef(r.stderr, "path arguments must not be blank\n")

					return request{}, 2
				}
			}
		}

		return request{command: command, args: args[1:]}, 0
	default:
		if command == "--go" || command == "--ts" || command == "--" {
			return r.parseFormatRequest(args)
		}

		if strings.HasPrefix(command, "-") {
			writef(r.stderr, "unknown option - {%q}\n\n", args[0])
			r.printUsage()

			return request{}, 2
		}

		return r.parseFormatRequest(args)
	}
}

func (r Runner) parseFormatRequest(args []string) (request, int) {
	req := request{command: "format", family: allLanguages}
	options := true

	for _, arg := range args {
		if options && arg == "--" {
			options = false

			continue
		}

		if options && (arg == "--go" || arg == "--ts") {
			family := goLanguage

			if arg == "--ts" {
				family = tsLanguage
			}

			if req.family != allLanguages && req.family != family {
				writef(r.stderr, "--go and --ts are mutually exclusive\n")

				return request{}, 2
			}

			req.family = family

			continue
		}

		if options && strings.HasPrefix(arg, "-") {
			writef(r.stderr, "unknown option - {%q}\n", arg)

			return request{}, 2
		}

		if strings.TrimSpace(arg) == "" {
			writef(r.stderr, "path arguments must not be blank\n")

			return request{}, 2
		}

		req.args = append(req.args, arg)
	}

	return req, 0
}
