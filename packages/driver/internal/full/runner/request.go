package runner

import (
	"strings"

	"github.com/go-playground/validator/v10"
)

type request struct {
	command string
	args    []string
}

type commandRequest struct {
	Command string `validate:"required,notblank,oneof=format format-all go ts check version --version -version help --help -h"`
}

type pathRequest struct {
	Paths []string `validate:"dive,notblank"`
}

var requestValidator = newRequestValidator()

func newRequestValidator() *validator.Validate {
	validate := validator.New()

	if err := validate.RegisterValidation("notblank", func(level validator.FieldLevel) bool {
		return strings.TrimSpace(level.Field().String()) != ""
	}); err != nil {
		panic(err)
	}

	return validate
}

func (r Runner) validateRequest(args []string) (request, int) {
	if len(args) == 0 {
		r.printUsage()

		return request{}, 2
	}

	command := strings.TrimSpace(args[0])

	if err := requestValidator.Struct(commandRequest{Command: command}); err != nil {
		writef(r.stderr, "unknown subcommand - {%q}\n\n", args[0])
		r.printUsage()

		return request{}, 2
	}

	req := request{command: command, args: args[1:]}

	if command == "format-all" && len(req.args) != 0 {
		r.printUsage()

		return request{}, 2
	}

	if command == "format" || command == "ts" {
		if err := requestValidator.Struct(pathRequest{Paths: req.args}); err != nil {
			writef(r.stderr, "path arguments must not be blank\n")

			return request{}, 2
		}
	}

	return req, 0
}
