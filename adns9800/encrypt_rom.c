#include <stdio.h>
#include <errno.h>
#include <stdlib.h>
#include <inttypes.h>
#define SIZE 3070
int main(int argc, char **argv){
	FILE *ifile = stdin;
	FILE *ofile = stdout;
	uint16_t state = 0, checksum = 0;
	if (argc < 2) {
		fprintf(stderr, "Missing key\n");
		exit(1);
	}
	else {
		state = strtol(argv[1], NULL, 16);
	}
	if (argc >= 3) {
		ifile = fopen(argv[2], "r");
	}
	if (argc == 4) {
		ofile = fopen(argv[3], "w");
	}
	if (!ifile || !ofile){
		perror("Error opening file");
		exit(1);
	}
	fputc(state >> 8, ofile);
	fputc(state&0xFF, ofile);
	for(int i = 0; i < SIZE - 4; i++){
		int in = fgetc(ifile);
		uint8_t nextbyte = (in == EOF ? 0 : in);
		if (i < SIZE - 6){
			checksum += nextbyte;
			checksum ^= 0xd8;
			if (checksum & 0x8000){
				checksum <<= 1;
				checksum += 0xe5;
			}
			else {
				checksum <<= 1;
			}
		}
		else if (i == SIZE - 6){
			nextbyte = checksum >> 8 ^ 0xbe;
		}
		else {
			nextbyte = (checksum & 0xFF) ^ 0xef;

		}
		state ^= 0xc1;
		if (state & 0x8000){
			state <<= 1;
			state += 0xe1;
		}
		else {
			state <<= 1;
		}
		nextbyte ^= state & 0xff;
		fputc(nextbyte, ofile);
	}
	state ^= 0xBEEF;
	fputc(state >> 8, ofile);
	fputc(state&0xFF, ofile);
}

