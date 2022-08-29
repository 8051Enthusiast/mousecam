#include <assert.h>
#include <inttypes.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <errno.h>
#include <unistd.h>

#include <hidapi/hidapi.h>

#define RECOVERY

#define USB_VID 0x258a
#define USB_PID 0x0012
#define USB_INTERFACE 1
#define USB_RECOVERY_PID 0xff12
#define USB_RECOVERY_INTERFACE 0

#define READ_SIZE 512

#define OFFSET 0

#define PACKET_SIZE 1024
#define PACKET_WRITE_SIZE 512
#define PACKET_RPNUM 5

#define check_output_error(err_msg, x) if(x) {fprintf(stderr, "%s\n", (err_msg)); return EXIT_FAILURE;}

const uint8_t RECOVERY_PACKET[16] = {0x02,0x03,0xaa,0xbb,0x01,0x00,0x00,0x00,
					0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00};
const uint8_t ROM_ERASE[16] = {0x02,0xf2,0x00,0x00,0x00,0x00,0x00,0x00,
				0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00};
const uint8_t ROM_ERASE_ANS[16] = {0x02,0xf2,0x00,0x00,0x00,0x00,0x00,0x00,
					0xf1,0x00,0x00,0x00,0x00,0x00,0x00,0x00};
const uint8_t MOUSE_RESTART_PACKET[16] = {0x02, 0xf5, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
					0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00};



int get_report(uint8_t *buf, const int len, hid_device *dHandle){
#ifndef SIM
	int err = hid_get_feature_report(dHandle, buf,
		                          len);
	if (err < 0) {
		fprintf(stderr, "Error on GET_REPORT: %i.\n", err);
		return 1;
	}
	const wchar_t *error_msg = hid_error(dHandle);
	if (error_msg) {
		fwprintf(stderr, L"Error in HIDAPI:\n\t%s", error_msg);
		return 1;
	}
#endif
	return EXIT_SUCCESS;
}
int set_report(const uint8_t *buf, const int len, hid_device *dHandle){
#ifndef SIM
	int err = hid_send_feature_report(dHandle, buf,
		                          len);
	if (err < 0) {
		fprintf(stderr, "Error on SET_REPORT: %i.\n", err);
		return 1;
	}
	const wchar_t *error_msg = hid_error(dHandle);
	if (error_msg) {
		fwprintf(stderr, L"Error in HIDAPI:\n\t%s", error_msg);
		return 1;
	}
#else
	printf("SET_REPORT:");
	for(int i = 0; i < len; i++){
		printf(" %02x", buf[i]);
	}
	printf("\n");
#endif
	return 0;
}

hid_device *get_device(int vid, int pid, int interface){
	struct hid_device_info *devices, *dev_iterator;
	devices = hid_enumerate(vid, pid);
	hid_device *devHandle;
	dev_iterator = devices;

	while (dev_iterator) {
		fwprintf(stderr, L"Path: %s\nSerial Number: %s\n", dev_iterator->path,
		         dev_iterator->serial_number);
		if (dev_iterator->interface_number == interface)
			break;
		dev_iterator = dev_iterator->next;
	}
	if (!dev_iterator) {
		fprintf(stderr, "Could not find device.\n");
		return NULL;
	}
#ifndef SIM
	devHandle = hid_open_path(dev_iterator->path);
	if (!devHandle) {
		fprintf(stderr, "Could not get descriptor for usb device.\n");
		return NULL;
	}
	const wchar_t *error_msg = hid_error(devHandle);
	hid_free_enumeration(devices);
	if (error_msg) {
		fwprintf(stderr, L"Error in HIDAPI:\n\t%s", error_msg);
		return NULL;
	}
#endif
	return devHandle;
}

int check_checksum(FILE *handle, int checksum){
	fprintf(stderr, "Argument checksum: %x\n",checksum);
	errno = 0;
	int err = fseek(handle, 0L, SEEK_SET);
	if(err){
		fprintf(stderr, "Can't seek: %s\n", strerror(errno));
		return 1;
	}
	unsigned char buf[READ_SIZE];
	unsigned int sum = 0;
	int num = fread(buf,1,READ_SIZE,handle);
	do{
		if(ferror(handle)){
			fprintf(stderr, "Error reading file\n");
			return 1;
		}
		for(int i = 0; i < num; i++){
			sum += buf[i];
			sum &= 0xffff;
		}
		num = fread(buf,1,READ_SIZE,handle);
	} while (!feof(handle));
	fprintf(stderr, "Calculated Checksum: %x\n", sum);
	errno = 0;
	err = fseek(handle, 0L, SEEK_SET);
	if(err){
		fprintf(stderr, "Can't seek: %s\n", strerror(errno));
		return 1;
	}
	return sum != checksum;
}

int send_file_to_rom(FILE *handle, hid_device *dHandle){
	fprintf(stderr, "Sending file to ROM\n");
	errno = 0;
	int err = fseek(handle, 0L, SEEK_SET);
	if(err){
		fprintf(stderr, "Can't seek: %s\n", strerror(errno));
		return -1;
	}
	char buf[PACKET_WRITE_SIZE];
	uint8_t packet[PACKET_SIZE];
	struct {
		uint8_t rpnum;
		uint8_t cmd;
		uint8_t addr[2];
		uint8_t len[2];
		uint8_t pad[2];
	} header;
	assert(sizeof(header) == 8);
	int address = ftell(handle)+OFFSET;
	assert(address >= 0 && address <= 0xeffd);
	int num = fread(buf,1,PACKET_WRITE_SIZE,handle);
	assert(address + num <= 0xeffd);
	do{
		if(ferror(handle)){
			fprintf(stderr, "Error reading file\n");
			return 1;
		}
		memset(packet, 0, PACKET_SIZE);
		header.rpnum = PACKET_RPNUM;
		header.cmd = 0xf3;
		header.addr[0] = address & 0xff;
		header.addr[1] = (address >> 8) & 0xff;
		header.len[0] = PACKET_WRITE_SIZE & 0xff;
		header.len[1] = (PACKET_WRITE_SIZE >> 8) & 0xff;
		header.pad[0] = 0;
		header.pad[1] = 0;
		memcpy(packet,&header,8);
		assert(8+num <= PACKET_SIZE);
		memcpy(packet+8,buf,num);
		err = set_report(packet, PACKET_SIZE, dHandle);
		if(err){
			fprintf(stderr, "Couldn't write packet at address %x\n", address);
			return 1;
		}
		address = ftell(handle)+OFFSET;
		assert(address >= 0 && address <= 0xeffd);
		num = fread(buf,1,PACKET_WRITE_SIZE,handle);
		assert(address + num <= 0xeffd);
	} while (!feof(handle));
	return 0;
}

int verify_checksum(int checksum, int length, hid_device *dHandle){
	fprintf(stderr, "Checking if Checksum matches\n");
	assert(length >= 3 && length <= 0xeffd);
	struct {
		uint8_t rpnum;
		uint8_t cmd;
		uint8_t len[2];
		uint8_t chksum[2];
		uint8_t pad[10];
	} sum_pkt;
	assert(sizeof(sum_pkt) == 16);
	sum_pkt.rpnum = 2;
	sum_pkt.cmd = 0xf4;
	sum_pkt.len[0] = length & 0xff;
	sum_pkt.len[1] = (length >> 8) & 0xff;
	sum_pkt.chksum[0] = checksum & 0xff;
	sum_pkt.chksum[1] = (checksum >> 8) & 0xff;
	memset(sum_pkt.pad, 0, sizeof(sum_pkt.pad));
	int err = set_report((char *)&sum_pkt,16,dHandle);
	if(err){
		fprintf(stderr, "Error while trying to verify checksum\n");
		return 1;
	}
	struct {
		uint8_t rpnum;
		uint8_t cmd;
		uint8_t jump[3];
		uint8_t len[2];
		uint8_t pad;
		uint8_t checksum[2];
		uint8_t tail[6];
	} check_buf = {0};
	check_buf.rpnum = 2;
	err = get_report((char*)&check_buf, 16, dHandle);
	if(err){
		fprintf(stderr, "Error while trying to get checksum information\n");
		return 1;
	}
	int ret_checksum = check_buf.checksum[0] | (check_buf.checksum[1]<<8);
	fprintf(stderr, "Checksum information:\n"
			"\tcmd: %x\n"
			"\tjump bytes: %x %x %x\n"
			"\tlength of checksum check: %d\n"
			"\tchecksum: %x\n"
			"\tlast 6 bytes: %x %x %x %x %x %x\n",
			check_buf.cmd,
			check_buf.jump[0], check_buf.jump[1], check_buf.jump[2],
			check_buf.len[0] | (check_buf.len[1]<<8),
			ret_checksum,
			check_buf.tail[0],check_buf.tail[1],check_buf.tail[2],check_buf.tail[3],
			check_buf.tail[4],check_buf.tail[5]);
#ifndef SIM
	if(ret_checksum != checksum){
		fprintf(stderr, "Error: Checksums do not match\n");
		return 1;
	}
#endif
	return 0;
}

int main(int argc,char** argv) {
	errno=0;
	int err = 0;
	FILE* handle;
	assert(argc == 3);
	handle = fopen(argv[1],"r");
	check_output_error("Error opening file", !handle);

#ifndef OVERRIDE_JUMP
	const uint8_t RIGHT_JUMP[3] = {0x02,0x53,0x0d};
	uint8_t actual_jump[3] = {0};
	err = fread(&actual_jump, 1, 3, handle);
	check_output_error("Can't read, like, at all", err < 3);
	check_output_error("First bytes are not a jump I approve of", memcmp(actual_jump, RIGHT_JUMP, 3));
#endif

	int checksum;
	err = sscanf(argv[2],"%i",&checksum);
	check_output_error("Please give the checksum as the second argument", err != 1)

	err = check_checksum(handle, checksum);
	check_output_error("Checksum does not match with argument, bailing out",err);

	fprintf(stderr, "Trying to set device into recovery mode...\n");
	hid_device *non_recovery = get_device(USB_VID,USB_PID,USB_INTERFACE);
	if(non_recovery){
		set_report(RECOVERY_PACKET, 16, non_recovery);
		fprintf(stderr, "Sent packet to invoke recovery\n");
	}
	sleep(5);


	hid_device *recovery = get_device(USB_VID, USB_RECOVERY_PID,
		    USB_RECOVERY_INTERFACE);
#ifndef SIM
	check_output_error("Bailing out...",!recovery)
#endif

	if(!non_recovery)
		fprintf(stderr, "Device was already in recovery mode\n");
	fprintf(stderr, "Proceed with writing to ROM? (y/N): ");
	char answer[16] = {0};
	fgets(answer,16,stdin);
	if(strncmp(answer, "y\n", 15)){
		return EXIT_FAILURE;
	}


	fprintf(stderr, "Erasing Rom\n");
	err = set_report(ROM_ERASE, 16, recovery);
	check_output_error("Error while erasing ROM", err);

	uint8_t erase_ans[16] = {0};
	erase_ans[0] = 2;
#ifndef SIM
	err = get_report(erase_ans, 16, recovery);
	if(err || memcmp(erase_ans, ROM_ERASE_ANS, 16)){
		fprintf(stderr, "Something went wrong after ROM erase\n"
				"GET_REPORT answer is:");
		for(int i = 0; i < 16; i++){
			fprintf(stderr, " %x", erase_ans[i]);
		}
		fprintf(stderr, "\n");
		return EXIT_FAILURE;
	}
#endif


	err = send_file_to_rom(handle, recovery);
	check_output_error("Error while writing to ROM",err);


	err = verify_checksum(checksum, ftell(handle), recovery);
	check_output_error("Error verifying checksum",err);


	fprintf(stderr,"Will now attempt to restart the mouse.\n"
			"If your changes cause an inability to access recovery mode, now is your chance to say no\n"
			"Do you want to restart the mouse and place the jump at the entry? Then type \"Yes I do!\": ");
	memset(answer, 0, 16);
	fgets(answer,16,stdin);
	fprintf(stderr, "%s\n", answer);
	if(strncmp(answer, "Yes I do!\n", 15)){
		return EXIT_FAILURE;
	}

	fprintf(stderr,"Will restart in");
	for(int i = 5; i > 0; i--){
		fprintf(stderr, " %d", i);
		sleep(1);
	}
	fprintf(stderr, " 0\n");


	err = set_report(MOUSE_RESTART_PACKET, 16, recovery);
	check_output_error("Could for some reason not restart the mouse",err);
#ifndef SIM
	for(int i = 0; i < 10; i++){
		sleep(5);
		non_recovery = get_device(USB_VID,USB_PID,USB_INTERFACE);
		if(non_recovery){
			fprintf(stderr, "Transfer seemed to work, mouse is now in non-recovery mode\n");
			return EXIT_SUCCESS;
		}
		recovery = get_device(USB_VID, USB_RECOVERY_PID,
			    USB_RECOVERY_INTERFACE);
		if(recovery){
			fprintf(stderr, "Device still in recovery mode\n");
			return EXIT_FAILURE;
		}
	}
	fprintf(stderr, "This mouse is not waking up anymore. I'm sorry for your loss.\n");
#define EXIT_UTTER_FAILURE EXIT_FAILURE
	return EXIT_UTTER_FAILURE;
#else
	return EXIT_SUCCESS;
#endif
}
