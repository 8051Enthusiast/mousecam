#include <assert.h>
#include <inttypes.h>
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <getopt.h>
#include <errno.h>


#ifndef SIM
#include <hidapi/hidapi.h>
#else
#define hid_device void
#endif

#define USB_VID 0x258a
#define USB_PID 0x0012

#define write_number_to_bytes(x, var)                                          \
    ((var)[0] = (x)&0xff, (var)[1] = (x) >> 8 & 0xff, (x)&0xffff)

#define get_number_from_bytes(var) (var[0] | var[1] << 8)

int opt_ver = 1;
/*
 * struct of a packet which sets the address and size of memory so it can be
 * fetched
 * in another request
 */
struct mouse_addr_pkt {
    unsigned char
        size_number;        /*dependent on packet size, for size_number > 1, the
                            packet size is: 4**size_number			*/
    unsigned char command;  /*is 5 for memory fetch address set		*/
    unsigned char magic[2]; /*meaning unknown, value always {bb,aa}
                             */
    unsigned char addr[2];  /*address of memory to be fetched		*/
    unsigned char size[2];  /*size of memory to be fetched  */
    unsigned char padding[8]; /*should be zero*/
} __attribute__((packed));

/*
 * Return header for memory fetch, data requested is after the header
 * the size is 4^size_number with size_number > 1, size - 8 should be
 * equal or bigger than the requested memory
 *
 * As this packet is received, only the size_number needs to be set,
 * as it is also the reportID for hidapi
 */
typedef struct {
    unsigned char size_number;
    unsigned char command;
    unsigned char addr[2];
    unsigned char size[2];
    unsigned char magic[2];
} __attribute__((packed)) mouse_read_hdr;

unsigned char *init_mouse_read_hdr(size_t data_size) {
    int sizenum = (int)ceil(log2((float)data_size + 8) / 2);
    int size = 1 << 2 * sizenum;
    assert(sizenum >= 2 && sizenum <= 6 && size > 0);

    unsigned char *ret = (unsigned char*)malloc(sizeof(unsigned char) * size);
    if (ret != NULL) {
        mouse_read_hdr *head = (mouse_read_hdr *)ret;
        head->size_number = (unsigned char)sizenum;
    }
    if(opt_ver > 1){
        fprintf(stderr,"Create mouse_read_hdr at %p with size %i from data_size %li:", ret,
               size, data_size);
    
        for (int i = 0; i < 8; i++)
            fprintf(stderr," %i", (int)ret[i]);
        fprintf(stderr,"\n");
    }
    return ret;
}

/*
 * Header for memory write, data is to be appended
 * the size to allocate is 4^size_number with size_number > 1, size - 8 should
 * be equal or bigger than the written memory
 *
 * magic is {0xbb,0xaa}
 */
typedef struct {
    unsigned char size_number;
    unsigned char command;
    unsigned char magic[2];
    unsigned char addr[2];
    unsigned char size[2];
} __attribute__((packed)) mouse_write_hdr;

unsigned char *init_mouse_write_hdr(size_t data_size) {
    int sizenum = (int)ceil(log2((float)data_size + 8) / 2);
    int size = 1 << 2 * sizenum;
    assert(sizenum >= 2 && sizenum <= 6 && size > 0);

    unsigned char *ret = malloc(sizeof(unsigned char) * (size));
    if (ret) {
        mouse_write_hdr *head = ((mouse_write_hdr *)ret);
        head->size_number = sizenum;
        head->command = 4;
        write_number_to_bytes(0xaabb, head->magic);
        memset(ret + 8, 0, size - 8);
    }
    if(opt_ver > 1){
        fprintf(stderr,"Create mouse_write_hdr at %p with size %i from data_size %li:", ret,
               size, data_size);
        for (int i = 0; i < 8; i++)
            fprintf(stderr," %x", (int)ret[i]);
        fprintf(stderr,"\n");
    }
    return ret;
}

/*
 * initializes the mouse_addr_pkt struct with mandatory values,
 * addr and size still have to be set
 */
void init_mouse_addr_pkt(struct mouse_addr_pkt *mouse_addr) {
    mouse_addr->size_number = 2;
    mouse_addr->command = 5;
    write_number_to_bytes(0xaabb, mouse_addr->magic);
    memset(mouse_addr->padding, 0, 8);
}

/*
 * Gets the data from the mouse's rom in the specified range
 * and outputs it to stdout
 */
int read_data(hid_device *dHandle, uint16_t packet_size, unsigned short min,
              uint16_t len, FILE* file) {
    assert(packet_size > 0);
    int max;
    if(len > 0)
      max = min + len - 1;
    else if(len == -1)
      max = 0xffff;
    else
      return EXIT_FAILURE;


    struct mouse_addr_pkt mouse_addr;
    init_mouse_addr_pkt(&mouse_addr);

    unsigned char *read_buf = init_mouse_read_hdr(packet_size);
    assert(read_buf);
    mouse_read_hdr *mouse_read = (mouse_read_hdr *)read_buf;
    unsigned char *read_data = read_buf + 8;
    int read_size = 1 << 2 * mouse_read->size_number;

    const wchar_t *error_msg;
    int err;

    assert (min <= max);
    for (int i = min; i <= max; i += packet_size) {

        if (max - i + 1 < packet_size)
            write_number_to_bytes(max - i + 1, mouse_addr.size);
        else
            write_number_to_bytes(packet_size, mouse_addr.size);
        write_number_to_bytes(i, mouse_addr.addr);

	if(opt_ver > 1)
        fprintf(stderr, "Read from %x with length %i\n",
                get_number_from_bytes(mouse_addr.addr),
                get_number_from_bytes(mouse_addr.size));
#ifndef SIM
        err =
            hid_send_feature_report(dHandle, (unsigned char *)&mouse_addr, 16);
        if (err < 0) {
            fprintf(stderr, "Error sending data: %i.\n", err);
            return EXIT_FAILURE;
        }
        error_msg = hid_error(dHandle);
        if (error_msg) {
            fwprintf(stderr, L"Error in HIDAPI:\n\t%s", error_msg);
            return EXIT_FAILURE;
        }
        err = hid_get_feature_report(dHandle, read_buf,
                                     read_size);
        if (err < 0) {
            fprintf(stderr, "Error sending data: %i.\n", err);
            return EXIT_FAILURE;
        }
        error_msg = hid_error(dHandle);
        if (error_msg) {
            fwprintf(stderr, L"Error in HIDAPI:\n\t%s", error_msg);
            return EXIT_FAILURE;
        }
	if(opt_ver > 1){
            fprintf(stderr,
                    "Returned: Address %x with length %i returned with data:",
                    get_number_from_bytes(mouse_read->addr),
                    get_number_from_bytes(mouse_read->size));
            for (int i = 0; i < get_number_from_bytes(mouse_read->size); i++) {
                fprintf(stderr, " %x", read_data[i]);
            }
            fprintf(stderr, "\n");
	}
        assert(mouse_read->size_number <= 4 && mouse_read->command == 3 &&
            get_number_from_bytes(mouse_read->magic) == 0xfafa);

        err = fwrite(read_data,1,get_number_from_bytes(mouse_read->size),
                     file);
        if (err == 0 || ferror(file)) {
            fprintf(stderr, "Error writing.\n", err);
            return EXIT_FAILURE;
        }
#endif
    }
    free(read_buf);
    return EXIT_SUCCESS;
}
int write_data(hid_device *dHandle, uint16_t packet_size,
                uint16_t min, unsigned short len, FILE* file) {
    fprintf(stderr, "Arguments given: %p, %i, %i, %i, %p", dHandle,
            packet_size, min, len, file);
    assert(packet_size > 0);
    int max;
    if(len > 0)
      max = min + len - 1;
    else if(len == -1)
      max = 0xffff;
    else
      return EXIT_FAILURE;

    unsigned char *write_buf = init_mouse_write_hdr(packet_size);
    assert(write_buf);

    mouse_write_hdr *mouse_write = (mouse_write_hdr *)write_buf;
    unsigned char *write_data = write_buf + 8;
    const wchar_t *error_msg;
    int write_size = 1 << 2 * mouse_write->size_number;
    int err;
    assert (min <= max);
    for (int i = min; i <= max; i += packet_size) {
        if (max - i + 1 < packet_size)
            write_number_to_bytes(max - i + 1, mouse_write->size);
        else
            write_number_to_bytes(packet_size, mouse_write->size);
	size_t size = get_number_from_bytes(mouse_write->size);
        write_number_to_bytes(i, mouse_write->addr);
        err = fread(write_data, 1, size, 
			file);
	if (opt_ver > 1)
	    fprintf(stderr, "Read %i bytes from file\n", err);
	if (err > 0 && err < size){
	    write_number_to_bytes(err, mouse_write->size);
	    size = err;
	}
	else if (!err){
	    if(ferror(file))
              fprintf(stderr, "Error reading the file on byte %i\n", i-min+1);
	    return EXIT_FAILURE;
	}
	if (size < packet_size)
	    memset(write_data+size,0,packet_size-size);
	if(opt_ver > 1){
            fprintf(stderr, "Writing to address %x with length %li:",
                    get_number_from_bytes(mouse_write->addr),
                    size);
            for (int i = 0; i < size; i++)
                fprintf(stderr, " %x", write_data[i]);
            fprintf(stderr, "\n");
	}
#ifndef SIM
        err = hid_send_feature_report(dHandle, write_buf,
                                      write_size);
        if (err < 0) {
            fprintf(stderr, "Error writing data: %i.\n", err);
            return EXIT_FAILURE;
        }
        error_msg = hid_error(dHandle);
        if (error_msg) {
	    fprintf(stderr, "HALLO");
            fwprintf(stderr, L"Error in HIDAPI:\n\t%s\n", error_msg);
            return EXIT_FAILURE;
        }
	if (ferror(file)){
	    fprintf(stderr, "Error reading the file on byte %li\n", i-min+size+1);
	}
#endif
    }
    free(write_buf);
    return EXIT_SUCCESS;
}

void usage(char* name) {
    const char *usage_text = "t3_dump: Dumps or writes contents of the T3 Wired Gaming Mouse\n"
    "\n"
    "Usage:\n"
    "\t%s [options]\n"
    "\n"
    "Options:\n"
    "\t-a, --address=N\t\tStart at address N of the flash of the rom\n"
    "\t-h, --help\t\tPrints this message\n"
    "\t-i, --infile=PATH\tUses PATH as input file for writing to rom\n"
    "\t-l, --length=N\t\tWrite or read at most N bytes\n"
    "\t-o, --outfile=PATH\tUses PATH as output file for reading from rom\n"
    "\t-r, --read\t\tRead from mouse\n"
    "\t-v, --verbose\t\tBe more verbose\n"
    "\t-w, --write\t\tWrite to mouse (requires address)\n"
    "\n"
    "If no input or output files are given, t3_dump will use stdin and stdout\n"
    "\n";
    printf(usage_text, name);
    exit(EXIT_FAILURE);
}

typedef struct {
    int write;
    int addr;
    char *filepath;
    int len;
} opt_con;

int check_args(opt_con opts){
    if (opts.write == 0 && opts.addr == -1 && opts.len == 0)
         return 1;
    if (opts.addr >= 0 && (opts.write == 0 || opts.write == 1)){
        if (opts.len == 0)
            return 0;
	    return 1;
    }
    return 0;
}

int main(int argc,char** argv) {
    int opt = 0;
    const char *opt_string = "a:hi:l:o:rvVw";
    const struct option long_opts[] = {
        { "--address", required_argument, NULL, 'a'},
        { "--help", no_argument, NULL, 'h'},
        { "--infile", required_argument, NULL, 'i'},
        { "--length", required_argument, NULL, 'l'},
        { "--read", no_argument, NULL, 'r'},
        { "--var-length", no_argument, NULL, 'V'},
        { "--verbose", no_argument, NULL, 'v'},
        { "--write", no_argument, NULL, 'w'},
        { 0,0,0,0 }
    };
    int long_index;
    opt_con opt_args;
    opt_args.write = -1;
    opt_args.addr = -1;
    opt_args.filepath = NULL;
    opt_args.len = 0;

    opt = getopt_long (argc, argv, opt_string, long_opts, &long_index);
    while(opt != -1){
        switch(opt){
        case 'r':
            if (opt_args.write == 1){
                fprintf(stderr,"Please only give one of -r and -w\n");
                exit(EXIT_FAILURE);
            }
            opt_args.write = 0;
            break;
        case 'w':
            if (opt_args.write == 0){
                fprintf(stderr,"Please only give one of -r and -w\n");
                exit(EXIT_FAILURE);
            }
            opt_args.write = 1;
            break;
        case 'a':
            errno = 0;
            char *ender;
            long int addr = strtol(optarg, &ender, 0);
            if(errno){
                fprintf(stderr,"Error while reading address: %s\n",strerror(errno));
                exit(EXIT_FAILURE);
            }
            if(*ender != '\0'){
                fprintf(stderr,"Error while reading address: not a number\n");
                exit(EXIT_FAILURE);
            }
            if(addr < 0 || addr > 0xffff){
                fprintf(stderr,"Error while reading address: not in range\n");
                exit(EXIT_FAILURE);
            }
            opt_args.addr = addr;
            break;
        case 'l':
            errno = 0;
            char *ender2;
            long int length = strtol(optarg, &ender2, 0);
            if(errno){
                fprintf(stderr,"Error while reading length: %s\n",strerror(errno));
                exit(EXIT_FAILURE);
            }
            if(*ender2 != '\0'){
                fprintf(stderr,"Error while reading length: not a number\n");
                exit(EXIT_FAILURE);
            }
            if(length <= 0 || length > 0x10000){
                fprintf(stderr,"Error while reading length: not in range\n");
                exit(EXIT_FAILURE);
            }
            opt_args.len = length;
            break;
        case 'o':
            errno = 0;
            if(opt_args.write == 0){
                opt_args.filepath = strdup(optarg);
                if(errno)
                    fprintf(stderr,"Error while reading outfile: %s\n",strerror(errno));
            }
            break;
        case 'i':
            errno = 0;
            if(opt_args.write == 1){
                opt_args.filepath = strdup(optarg);
                if(errno)
                    fprintf(stderr,"Error while reading outfile: %s\n",strerror(errno));
            }
            break;
        case 'V':
            if (opt_args.len == 0)
                opt_args.len = -1;
            break;
        case 'h':
        case '?':
            usage(argv[0]);
            break;
        case 'v':
            opt_ver += 1;
            break;
        default:
            break;
        }
        opt = getopt_long (argc, argv, opt_string, long_opts, &long_index);
    }
    if(!check_args(opt_args)){
        fprintf(stderr, "Error: Inconsistent options\n");
        return EXIT_FAILURE;
    }
#ifdef SIM
    void *devHandle = NULL;
#else
    struct hid_device_info *devices, *dev_iterator;
    devices = hid_enumerate(USB_VID, USB_PID);
    hid_device *devHandle;
    dev_iterator = devices;

    while (dev_iterator) {
        fwprintf(stderr, L"Path: %s\nSerial Number: %s\n", dev_iterator->path,
                 dev_iterator->serial_number);
        if (dev_iterator->interface_number == 1)
            break; /* choose usb interface 1*/
        dev_iterator = dev_iterator->next;
    }
    if (!dev_iterator) {
        fprintf(stderr, "Could not find device.\n");
        return EXIT_FAILURE;
    }
    devHandle = hid_open_path(dev_iterator->path);
    if (!devHandle) {
        fprintf(stderr, "Could not get descriptor for usb device.\n");
        return EXIT_FAILURE;
    }
    const wchar_t *error_msg = hid_error(devHandle);
    hid_free_enumeration(devices);
    if (error_msg) {
        fwprintf(stderr, L"Error in HIDAPI:\n\t%s", error_msg);
        return EXIT_FAILURE;
    }
#endif
    if(opt_args.write == 1){
        errno = 0;
        FILE* handle = stdin;
        if(opt_args.filepath){
            handle = fopen(opt_args.filepath,"r");
            if (errno){
                fprintf(stderr,"Error opening %s: %s\n",opt_args.filepath,strerror(errno));
                return EXIT_FAILURE;
            }
        }
        write_data(devHandle, 56, opt_args.addr, opt_args.len, handle);
    } else if(opt_args.write == 0){
        errno = 0;
        FILE* handle = stdout;
        if(opt_args.filepath){
            handle = fopen(opt_args.filepath,"w+");
            if (errno){
                fprintf(stderr,"Error opening %s: %s\n",opt_args.filepath,strerror(errno));
                return EXIT_FAILURE;
            }
        }
        read_data(devHandle, 56, opt_args.addr, opt_args.len, handle);

    }
    return EXIT_SUCCESS;
}
