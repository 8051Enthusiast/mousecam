	ORG 0C006h
	RELOC_DEST	EQU	08000h
; for getting out of interrupt
ENTRY:
        PUSH ACC
        PUSH DPL
        PUSH DPH
	; this register contains the address
	; to continue after the interrupt
	; (it is placed at the bottom of the stack
	; so that it is executed sometime after the interrupt)
        MOV DPTR, #08010h
        MOV A, #(LOW MAIN)
        MOVX @DPTR, A
        INC DPTR
        MOV A, #(HIGH MAIN)
        MOVX @DPTR, A
	INC DPTR
        MOV A, #(LOW MAIN)
        MOVX @DPTR, A
        INC DPTR
        MOV A, #(HIGH MAIN)
        MOVX @DPTR, A
        MOV DPTR, #0802Ah
        MOVX A, @DPTR
        ORL A, #080h
        MOVX @DPTR, A
        POP DPH
        POP DPL
        POP ACC
        RET
; main loader function
MAIN:
	CLR IE.7
        ANL PCON, #0EFh
        MOV SP, #08h
	MOV 084h, #00h
	MOV 0F3h, #00h

; copy the main segment to 0x8000
	MOV DPTR, #MAIN_SEG_START
	MOV R7, #(LOW RELOC_DEST)
	MOV R6, #(HIGH RELOC_DEST)
RELOC_LOOP:
	MOV 0B7h, #0FFh
	CLR A
	MOVC A, @A+DPTR
	INC DPTR
	MOV R5, DPL
	MOV R4, DPH
	MOV DPL, R7
	MOV DPH, R6
	MOVX @DPTR, A
	INC DPTR
	MOV R7, DPL
	MOV R6, DPH
	MOV DPL, R5
	MOV DPH, R4
	CJNE R5, #(LOW MAIN_SEG_END), RELOC_LOOP
	CJNE R4, #(HIGH MAIN_SEG_END), RELOC_LOOP
	LJMP RELOC_DEST
MAIN_SEG_START:
$INCLUDE(main.bin.a51)
MAIN_SEG_END:
	END
