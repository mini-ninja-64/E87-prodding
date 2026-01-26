# RSCP DATA


paramLength + 8

| 0xFE [1] | 0xDC [1] | 0xBA [1] | packetMetadata [1] | opCode [1] | commandOrResponseLength [2] | COMMAND_OR_RESPONSE_DATA | 0xEF [1] |

COMMAND_DATA (bytes):
    if opCode == 1:
        | op code sn [1] | Xm Op Code [1] | param data [N] |
    else:
        | op code sn [1] | param data [N] |

RESPONSE_DATA (bytes):
    if opCode == 1:
        | status [1] | op code sn [1] | Xm Op Code [1] | param data [N] |
    else:
        | status [1] | op code sn [1] | param data [N] |

getType == 1:
 0b10000000
 0b01000000
 
packet_metadata (1 byte): | type [1] | commandHasResponse [1] | unknown [6] |

type == 1 // command
type == 0 // response

0xC0 == command and expects response
0x00 == response with no expectation of response
0x80 == command with no expectation of response

upload image:

| 00 00 0a b9 | 32 af | 35 64 61 61 30 63 66 64 2e 74 6d 70 00 | ef |
| size        | crc16 | 5daa0cfd.tmp\null                      |    |

| 00 | 0f 50         | 00 00 01 ea  | ef |
| op | buffer (3920) | offset (490) |    |



```java
public static void main(String[] args) {
    String fileName = "my_file.jpeg";
    long lastModified = 1234567L;
    String toHash = fileName + lastModified;
    System.out.println("toHash: " + toHash);
    
    String hashCodeAscii = String.format("%08x", toHash.hashCode());
    
    hashCodeAscii += ".tmp\u0000";
    
    System.out.println(hashCodeAscii);
}
```
3920 chunking, but whyyy??
