

push:
	@git add .
	@git commit -am update || true
	@git push
