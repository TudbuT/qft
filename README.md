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
```sh
echo 'alias qftr="qft receiver tudbut.de:4277"' >> ~/.(shell)rc
echo 'alias qfts="qft sender tudbut.de:4277"' >> ~/.(shell)rc
```

## Cool stuff
- Files are transferred over UDP, but qft has additional reliability measures in place to avoid
  broken files.
- Unreliable internet connection? No problem! QFT will simply pause transmission until the
  connection is back! Doesn't work? Check out the "Resume a fully stopped transfer" section!
- Did you know you can hibernate or suspend your computer while it's transferring and it'll continue
  where it left of, even when only one side is suspended (unless your router blocked the port, read 
  the "Resume a fully stopped transfer" section in that case)

## Resume a fully stopped transfer
You most likely won't need this unless the transfer completely died due to a VERY long pause or a
computer restart, but if you do:

Stop qft on both ends and start it again with the [skip] parameter in place (if you didn't specify a
bitrate before, the default is 256). It will skip those bytes and continue where you specified.

## Troubleshooting

### It constantly says `CONNECTING`
One of your ends didn't correctly connect to the helper. Stop the transfer on both ends
and try again. If it still doesn't work, make SURE the time and date on both ends are within an
error of <0.1 seconds! Holepunching strongly relies on the time and date matching. (If you have any
suggestion on how I can mitigate this reliance on time, please open an issue!)
