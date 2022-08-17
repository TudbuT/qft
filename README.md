# qft
QFT is a small command line application for Quick Peer-To-Peer UDP file transfer.

## If a friend sent you here...

...look at the "Releases" section on the sidebar. You should see a link titled vX.Y.Z. Click on
that, and then choose the right file for your OS: `qft` for Linux, `qft-mac` for Mac, and `qft.exe`
for Windows. Download this file, make it executable in case of Linux or Mac, and then follow your 
friend's instructions on how to receive the file they wanted to send you.

## Usage:
- Find a public QFT helper (for example tudbut.de:4277)
- On the sender PC, enter `qft sender <helper> <shared-phrase> <filename>`.
- On the receiver PC, enter `qft receiver <helper> <shared-phrase> <filename>`.
- Both PCs should start transferring after a short while. If they don't, try again.

### Arguments:
```
qft helper   <bind-port>
qft sender   <helper-address>:<helper-port> <phrase> <filename> [bitrate] [skip]
qft receiver <helper-address>:<helper-port> <phrase> <filename> [bitrate] [skip]
```

## What helpers do

Helpers are NOT relays for data, they are only used to ESTABLISH the connection.

Helpers are there to help with holepunching.
- P1 connects\* to helper
- P1 sends the phrase to the helper
- P1 waits for a response
- Some time passes
- P2 connects\* to the same helper
- P2 sends the phrase to the helper
- P2 gets P1's public IP and port
- P1 gets P2's public IP and port
- P1 and P2 disconnect\* from the helper
- P1 and P2 start a loop (slightly simplified):
  - fire a packet at eachother multiple times
  - try to receive as many packets from the other one
  - if none are received, loop again
  - if one is received, exit the loop
- Connection between P1 and P2 is established.

\*UDP is a connection-less protocol, there are no handshakes. The word "connection" is used here as
an indicator that data will be exchanged between the "connected" parties. The word "disconnect" is used
here as an indicator that no more data will be exchanged between the "previously connected" parties.

## Tips 'n Tricks
- You can add a number to the end of both of your commands (after the filename) to
  boost transfer speeds (higher = faster), but a too large number might cause unreliability
  due to local network conditions or VPNs. The maximum possible is 65533 (65535 - 2).
- Helpers don't **have to** be run on a public server, they work in LAN too, but that way, only
  computers in the same LAN will be able to use them.
- You can allow streaming (for example when you want to transmit from /dev/stdin) by setting
  the `QFT_STREAM` environmental variable.
- To use qfts and qftr aliases on linux or mac, run (replacing `(shell)` with your shell name,
  usually bash or zsh):
```sh
echo 'alias qftr="qft receiver tudbut.de:4277"' >> ~/.(shell)rc
echo 'alias qfts="qft sender tudbut.de:4277"' >> ~/.(shell)rc
source ~/.(shell)rc
```

## Cool stuff
- Files are transferred over UDP, but qft has additional reliability measures in place to avoid
  broken files.
- Unreliable internet connection? No problem! QFT will simply pause transmission until the
  connection is back! Doesn't work? Check out the "Resume a fully stopped transfer" section!
- Did you know you can hibernate or suspend your computer while it's transferring and it'll continue
  where it left of, even when only one side is suspended? (Unless your router blocked the port, read 
  the "Resume a fully stopped transfer" section in that case)
- It's written in *100% pure Rust*.

## Resume a fully stopped transfer
You most likely won't need this unless the transfer completely died due to a VERY long pause or a
computer restart, but if you do:

Stop qft on both ends and start it again with the [skip] parameter in place (if you didn't specify a
bitrate before, the default is 256). It will skip those bytes and continue where you specified.

## Troubleshooting

### It says `Connecting...` but doesn't connect
One of your ends didn't correctly connect to the helper. Stop the transfer on both ends
and try again.

## Croc

Many people have mentioned how this is like croc. It isn't, because croc uses a relay that all your
data is sent through. This is a bottleneck and also means that the relay admins are responsible for
the content that is sent. The relay also buffers a lot of data, meaning its RAM might fill up if the
sender's connection is much faster than the receiver's.

## [Relevant XKCD](https://xkcd.com/949)

![Relevant XKCD Image](https://imgs.xkcd.com/comics/file_transfer.png)
