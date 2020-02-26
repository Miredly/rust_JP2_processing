# rust_JP2_processing
Rust port of the Python JP2_processing utility for making timelapse videos out of SDO data. Requires FFMPEG.

Expects 3 arguments: 
 
  A directory of AIA JP2 images organized according to spectrum
  
  A directory to output the finished video to
  
  A multiplier for the speed of the final video (eg, 1 will use all the frames, 2 every other frame, 3 every 3rd frame, etc.)
  
Ex: 

./rust_JP2_processing testdata ./ 1

Will go through appropriately named spectrum directories in the testdata directory, use every image in these directories, 
and output a finished video to the directory from which it's being run.
