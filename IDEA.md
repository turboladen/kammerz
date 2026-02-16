## Role

I'm a film photographer and would like you to help build an app that helps me catalog my rolls of
film. 

## Background

I like to catalog each roll of film I take, so I can know a bunch of information about shots and can
later, when viewing shot, I can go "what camera and film and settings did I use for that?" I used to
use Apple Notes for tracking this info, as it was easy to pull up on my phone when I was shooting
and didn't require a lot of clicks/taps and navigating to get to where I could quickly enter the
info I wanted to capture. I'm not necessarily looking to build a mobile app to fulfill that
purpose--in fact, I'll probably keep using that or NotePlan for the same purpose: quick note
jotting, then I'll transfer that info to the app that I wanna build here, which will probably be
either a desktop app or webapp (probably desktop).

After I get film developed (or develop myself), I process them (usually in Adobe Lightroom Classic),
then import them into Apple Photos, where I tag, date, and add locations if possible.

## The Problem

Historically I used a system for identifying cameras+rolls of [camera prefix]-[increasing integer
per roll], ex. a roll with ID "MD7-13" was shot with camera of ID `MD7`, which I gave to my Minolta
XD-7, and it was the 13th roll I've shot with that camera. This worked fine until I did a shoot
where I brought a few cameras and lost track of which roll I shot in which order and with which
camera. Since I couldn't easily determine that and I wanted to stick to my system, I told myself I'd
try to stitch together events of the day and determine which camera I used with which roll and in
which order. ...but I never got around to it, and because I amassed such a backlog of developed and
undeveloped negatives that I didn't know how to ID them, I just stopped taking pictures altogether.

A lesser issue was that in my use of Apple Notes for capturing all the info, I obviously wasn't able
to capture and search for specifics easily due to the unstructured nature of that app. I'd love to
have structured data that I can query easily.

I want to get back to taking pictures, but I need a better system for cataloging rolls. I want your
help adopting a system that I can capture in a simple Notes app, then transfer to the app that we'll
build.

## What I (Think I) Want To Track

Ideally I'd track the following, but in practice it often ends up that I don't want to have to write
down everything I've done after each shot--I want to enjoy just taking shots--so I'll wait till I'm
done, but then forget or don't have time in the moment. In my note for that roll, I'll often adding
a note like "Shot ~10-20 in early October", so I can add dates to the scans later.

### Roll Level Info

- Roll ID (ex. MD7-13)
- What camera I used with the roll
- What film type it was (ex. Kodak Portra 400)
- Any other roll-level notes

### Shot Level Info

- Which lens(es) I may have used per shot (optional)
- What settings I may have used per shot (optional, but preferable for when shooting large format)
- The date I took the picture
- The location I took the picture
- Any other shot-level notes

### Development Info

If I had a lab do the development, I just want to capture:

- Which lab
- The date I took them in
- The date I picked up the negatives or received them back in the mail (this serves as a reminder
  for me to pick up the negatives from the local lab if I forget)

If I develop myself, I want to capture:

- The date I processed
- Which developer I used
- Which fixer
- Which stop bath
- Which wetting agent (ex Kodak Photo-Flo)
- Which clearing agent (ex Kodak Hypo-Clearing)
- How long I took for each stage
- Maybe other things that I can't remember
