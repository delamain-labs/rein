package tree_sitter_rein_test

import (
	"testing"

	tree_sitter "github.com/smacker/go-tree-sitter"
	"github.com/tree-sitter/tree-sitter-rein"
)

func TestCanLoadGrammar(t *testing.T) {
	language := tree_sitter.NewLanguage(tree_sitter_rein.Language())
	if language == nil {
		t.Errorf("Error loading Rein grammar")
	}
}
