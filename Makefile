

open-v1:
	@old="$$(stty size)"; trap 'set -- $$old; stty rows "$$1" cols "$$2"' EXIT; stty rows 25 cols 80; dosemu -t -K "$$PWD/dos/EDIT/V1" -E EDIT.COM

open-v2:
	@old="$$(stty size)"; trap 'set -- $$old; stty rows "$$1" cols "$$2"' EXIT; stty rows 25 cols 80; dosemu -t -K "$$PWD/dos/EDIT/V2" -E EDIT.COM

push:
	@git add .
	@git commit -am update || true
	@git push

install:
	@cargo install --path .
