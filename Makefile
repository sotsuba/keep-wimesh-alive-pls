.PHONY: setup

setup:
	git config core.hooksPath .githooks
	@echo "Git hooks configured."
