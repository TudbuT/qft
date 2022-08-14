# qft
QFT is a small command line application for Quick Peer-To-Peer UDP file transfer.

## Usage:
- Find a public QFT helper (for example tudbut.de:4277)
- On the sender PC, enter `qft sender <helper> <shared-phrase> <filename>`.
- On the receiver PC, enter `qft receiver <helper> <shared-phrase> <filename>`.
- Both PCs should start transferring after a short while. If they don't, try again.

## Tips 'n Tricks
- You can add a number to the end of both of your commands (after the filename) to
  boost transfer speeds (higher = faster), but a too large number might cause unreliability
  due to local network conditions or VPNs.
- To use qfts and qftr aliases on linux or mac, run:
  run this:
```sh
echo 'alias qftr="qft receiver tudbut.de:4277"' >> ~/.bashrc
echo 'alias qfts="qft sender tudbut.de:4277"' >> ~/.bashrc
```

## Troubleshooting

### It constantly says `CONNECTING`
One of your ends didn't correctly connect to the helper. Stop the transfer on both ends
and try again.
