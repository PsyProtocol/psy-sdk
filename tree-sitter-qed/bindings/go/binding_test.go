package tree_sitter_qed_test

import (
	"testing"

	tree_sitter "github.com/tree-sitter/go-tree-sitter"
	tree_sitter_qed "github.com/tree-sitter/tree-sitter-qed/bindings/go"
)

func TestCanLoadGrammar(t *testing.T) {
	language := tree_sitter.NewLanguage(tree_sitter_qed.Language())
	if language == nil {
		t.Errorf("Error loading Qed grammar")
	}
}
