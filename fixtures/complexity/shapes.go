package shapes

func ifChain(a, b, c int) int {
	if a > 0 {
		return 1
	}

	if b > 0 {
		return 2
	}

	if c > 0 {
		return 3
	}

	return 0
}

func elseIfLadder(a int) string {
	if a == 1 {
		return "one"
	} else if a == 2 {
		return "two"
	} else if a == 3 {
		return "three"
	} else {
		return "other"
	}
}

func switchFour(a string) int {
	switch a {
	case "a":
		return 1
	case "b":
		return 2
	case "c":
		return 3
	case "d":
		return 4
	default:
		return 0
	}
}

func nestedClosure() func(int) int {
	return func(item int) int {
		if item > 0 {
			return item
		}

		return 0
	}
}

func logicalRun(a, b, c, d bool) bool {
	return a && b && c && d
}

func mixedLogical(a, b, c bool) bool {
	return (a && b) || c
}

func loopWithIf(items []int) int {
	total := 0

	for _, item := range items {
		if item > 0 {
			total += item
		}
	}

	return total
}

func (s *shape) Read() int {
	return 1
}

type shape struct{}
