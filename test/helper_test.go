package integration_test

import (
	"errors"
	"math/rand"
)

// randomData generates random string with fixed size.
func randomData() string {
	var letterRunes = []rune("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ")

	generator := func(n int) string {
		b := make([]rune, n)
		for i := range b {
			b[i] = letterRunes[rand.Intn(len(letterRunes))]
		}
		return string(b)
	}
	return generator(5)
}

// interfaceSlicetoStringSlice try to convert []interface to []string.
func interfaceSlicetoStringSlice(s []interface{}) ([]string, error) {
	ss := make([]string, len(s))
	for i, v := range s {
		if str, ok := v.(string); ok {
			ss[i] = str
		} else {
			return nil, errors.New("failed to parse interface{} to string")
		}
	}
	return ss, nil
}

// stringSliceToInterfaceSlice converts []string to []interface.
func stringSliceToInterfaceSlice(s []string) []interface{} {
	is := make([]interface{}, len(s))
	for i, v := range s {
		is[i] = v
	}
	return is
}
