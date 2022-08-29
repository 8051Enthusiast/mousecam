ASEM = asem
HEXBIN = hexbin

firmware: loader.bin
	cp loader.bin firmware

main.hex: main.a51 huff_table.a51 canon_buffman.a51
	$(ASEM) $< $@

%.hex: %.a51 $(DEPS)
	$(ASEM) $< $@

%.bin: %.hex
	$(HEXBIN) $< $@

%.bin.a51: %.bin
	hexdump -v -e '"db " /1 "0%02xh" "\n"' $< > $@

loader.hex: loader.a51 main.bin.a51
	$(ASEM) $< $@
