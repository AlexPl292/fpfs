# FPFS Architecture and concepts

## Overview

FPFS is a filesystem that uses Telegram Cloud Chats as a backend to store files, file blocks and it's metadata.
It's designed to be scallable and workaround native Telegram file size limitations.

FPFS stores all of the filesystem data in the cloud, so it's safe to looze all of the local state and configuration.

FPFS strong sides are:
- Built-in geo-redundancy and global distrubution since Telegram Cloud strores files this way natively.
- Zero cost for storage, ingress and egress except for FS client traffic.

FPFS weak sides:
- Slow read/write I/O since every file and it's block must be uploaded/downloaded from the Cloud
- I/O rate is limited with Telegram API request rate (approx. 100 req/s per client)

Possible use cases:
- Cold archive storage for keeping historical data
- Unfrequent read/write archive storage
- Low speed file syncrinization solution

Main challanges:
- Concurent read/write access to the same filesystem
- Local cache to speedup client I/O
- Telegram native file size limit
- Telegram native message size limit
- Telegram message editing timeout
- I/O slowdown on FS growth is linear (?)

## Entities

To support common UNIX FS features, FPFS use the following entities:

- File / Directory
- File block
- File inode
- Filesystem index

All of the entities abowe are represented as messages in a Telegram Chat.
