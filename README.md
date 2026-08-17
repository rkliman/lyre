# Lyre
A powerful and easy to use tui-based music player and library manager

## Features
- **Fully featured library system.** Allows you to navigate your music library through the full song list, albums, artists, genres, and playlists
- **Fully featured queue system.** Select tracks, add them to the queue, and play them. Queue can be modified at any time.
- **Album art.** Can render high-res album art if your terminal supports it (Sixel, Kitty, etc.), but also renders a pixelated version if not.
- **Shuffling/Looping playback.**
- **Integration with MPRIS on supported machines.**
- **Lyrics support.** View lyrics for songs if they have lyrics metadata, and even sing along to your favorite song if your lyrics are in the LRC lyrics format
- **Configurable color scheme.** Use the example toml file to set custom colors if you desire. Default color scheme is a brassy yellow.
- **Playlist management.** Loads .m3u playlist files and can edit them from within the app. Add or remove tracks from your playlists or even create new ones!

## Screenshots
<img width="3840" height="2054" alt="image" src="https://github.com/user-attachments/assets/e8eb04c2-1809-4a08-94f0-5496ebbfa653" />


## TODOs
- [X] Make color customization work. DONE! Don't use RGBA, use RGB
- [X] Add looping and shuffling capability
- [ ] Integrate apollo-music indexing on startup (make apollo-music a prepreq. does require apollo-music to be semi-finalized)
- [ ] Add better README.md documentation
- [ ] Far future: integrate with streamrip/other music acquisition to make it all-in-one
- [ ] Far future: add youtube integration
