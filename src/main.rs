
extern crate image;
extern crate imageproc;
extern crate palette;
extern crate rusttype;
extern crate glob;
extern crate jpeg2000;
extern crate rayon;
extern crate voca_rs;

use std::process::Command;
use std::sync::{Mutex};
use std::env;
use std::fs;
use std::io::{BufRead, BufReader, Read};
use imageproc::drawing::draw_text_mut;
use image::{GenericImage, GenericImageView, Rgba};
use image::imageops::overlay;
use rusttype::{FontCollection, Scale};
use glob::glob;
use rayon::prelude::*;
use voca_rs::*;

#[derive(Clone, Debug)]
struct MetaData{
	date: String,
	time: String,
	hour: String,
	wlen: String
}

#[derive(Clone)]
struct Frame {
	frm: image::DynamicImage,
	idx: u32,
	dat: MetaData
}

#[derive(Clone, Debug)]
struct Template {
	mus_id:         String,
	font:           String,
	input_dir:      String,
	output_dir:     String,
	
	resolution:     (u32, u32),
	
	ts:             (u32, u32),
	sun:            (u32, u32),
	earth:          (u32, u32),
	
	thumb94:        (u32, u32),
	thumb335:       (u32, u32),
	thumb211:       (u32, u32),
	thumb193:       (u32, u32),
	thumb171:       (u32, u32),
	thumb304:       (u32, u32),

	overlay:        (u32, u32),
	
	thumb_size:     u32,
	skip_frames:    u8
}

//create a list of filepaths to unprocessed image data
fn build_list(wlen: String) -> Vec<String>{
	let mut return_vec : Vec<String> = vec!("init".to_string());
	return_vec.pop(); 

	for entry in glob(&("".to_owned() + &wlen + "/*.jp2")).expect("Failed to read glob pattern"){
		match entry{
			Ok(path) => return_vec.push(path.display().to_string()),
			Err(e)   => println!("{:?}", e),
		}
	}
	return return_vec;
}

//remove every nth element of the path list
fn dec_list(list: Vec<String>, skip: u8) -> Vec<String>{
	let mut olist = Vec::new();
	for (i, f) in list.iter().enumerate(){
		if i as u8 % skip == 0{
			olist.push(f.clone());
		}
	}
	return olist;
}

//sort a list of frames by date / time in metadata
fn sort(mut flist : Vec<Frame>) -> Vec<Frame>{
	println!("SORTING... {}", flist[0].dat.wlen);
	flist.sort_by(|a, b| a.idx.cmp(&b.idx));
	return flist;
}

//parse metadata for each frame from the filename
fn parse_meta(file: String) -> MetaData {

	let split = file.split("__").collect::<Vec<_>>();
	let data  = MetaData{
		date: split[0].to_string().replace("_", "/"),
		time: split[1].to_string().replace("_", ":"),
		hour: split[1].split("_").collect::<Vec<_>>()[0].to_string(),
		wlen: split[2].split("_").collect::<Vec<_>>()[3].split(".").collect::<Vec<_>>()[0].to_string()
	};
	return data
}

fn tuple_from_string(string: String) -> (u32, u32){
	let string = string.replace(" ", "");	
	return(
		string.split(",").collect::<Vec<_>>()[0].replace("(", "").parse::<u32>().unwrap(),
		string.split(",").collect::<Vec<_>>()[1].replace(")", "").parse::<u32>().unwrap()
	);
}

fn open_template(path: &String) -> Template{
	let file                      = fs::File::open(path).unwrap();
	let reader                    = BufReader::new(file);
	let mut stripped: Vec<String> = Vec::new();
	
	let mut template = Template{
		mus_id:      "test".to_string(),
		font:        "test".to_string(),
		input_dir:   "test".to_string(),
		output_dir:  "test".to_string(),
		
		resolution:  (0, 0),
		
		ts:          (0, 0),
		sun:         (0, 0),
		earth:       (0, 0),

		thumb94:     (0, 0),
		thumb335:    (0, 0),
		thumb211:    (0, 0),
		thumb193:    (0, 0),
		thumb171:    (0, 0),
		thumb304:    (0, 0),

		overlay:     (0, 0),
		
		thumb_size:  0,
		skip_frames: 0
	};

	//strip any comment lines /newlines from the template
	for (_index, line) in reader.lines().enumerate(){
		let line = line.unwrap();
		println!("{:?}", line);
	    if !line.as_str().contains("#"){
	    	if line != "" {
	    		stripped.push(line.to_string());
	    	}
	    }
	}
	println!("{:?}", stripped);
	for (index, line) in stripped.iter().enumerate(){
		let line = line.to_string();
		match index{
			0  => template.mus_id      = line,
			1  => template.font        = line,
			2  => template.input_dir   = line,
			3  => template.output_dir  = line,
			
			4  => template.resolution  = tuple_from_string(line),
			
			5  => template.ts          = tuple_from_string(line),
			6  => template.sun         = tuple_from_string(line),
			7  => template.earth       = tuple_from_string(line),

			8  => template.thumb94     = tuple_from_string(line),
			9  => template.thumb335    = tuple_from_string(line),
			10 => template.thumb211    = tuple_from_string(line),
			11 => template.thumb193    = tuple_from_string(line),
			12 => template.thumb171    = tuple_from_string(line),
			13 => template.thumb304    = tuple_from_string(line),

			14 => template.overlay     = tuple_from_string(line),
			
			15 => template.thumb_size  = line.parse::<u32>().unwrap(),
			16 => template.skip_frames = line.parse::<u8>().unwrap(),
			 _ => (),
		}
	}

	return template;
}

fn open_jp2(path: String) -> image::DynamicImage{
    let codec      = jpeg2000::decode::Codec::JP2;
    let colorspace = jpeg2000::decode::DecodeConfig{ 
    	default_colorspace: Some(jpeg2000::decode::ColorSpace::SRGB), 
    	discard_level: 0};
    
    return jpeg2000::decode::from_file(path, codec, colorspace, None).unwrap();
}

//apply a given color lookup table to a grayscale image
fn apply_clut(mut img: image::DynamicImage, clut: image::DynamicImage) -> image::DynamicImage{
	let clut_len : u32 = clut.dimensions().0;

	for x in 0..img.dimensions().0{
		for y in 0..img.dimensions().1{
			let grey_pix = img.get_pixel(x, y);
			let clut_pix = clut.get_pixel((grey_pix[0] as f32 / 256 as f32 * clut_len as f32).round() as u32 , 0);
			img.put_pixel(x, y, clut_pix);
		}
	}
	return img;
}

fn annotate(mut frame: image::DynamicImage, text: String, fontpath: String, loc: (u32, u32), size: f32, color: (u8, u8, u8, u8)) -> image::DynamicImage{
	
	// let font   = Vec::from(include_bytes!("../media/misc/BebasNeue-Regular.ttf") as &[u8]);
	let mut fdata  = Vec::new();
	fs::File::open(fontpath).unwrap().read_to_end(&mut fdata).unwrap();
    let font   = FontCollection::from_bytes(fdata).unwrap().into_font().unwrap();
    let height = size;
    let scale  = Scale { x: height * 2.0, y: height };

    draw_text_mut(
    	&mut frame,
    	Rgba([color.0, color.1, color.2, color.3]),
    	loc.0, loc.1,
    	scale,
    	&font,
    	&text);

    return frame;
}

fn main(){
	let _ = fs::create_dir("./tmp");
	//clear tmp
	Command::new("sh")
		.arg("-c")
		.arg(format!("rm -r tmp/*.png"))
		.spawn()
		.expect("failed to execute process");

	let args: Vec<String> = env::args().collect();  
	let tp                = open_template(&args[1]);
	let target_dir        = tp.input_dir.to_string();
	let output_dir        = tp.output_dir.to_string();
	let skip_frames       = tp.skip_frames;
	
	let wavlist           = ["94", "335", "211", "193", "171", "304"];
	let mut wlist         = Vec::new(); 

	for wlen in wavlist.iter(){
		
		let list = build_list(target_dir.to_string() + "/" + wlen);
		let list = dec_list(list, skip_frames);

		let mut indices : Vec<u32> = Vec::new();
		for i in 0..list.len(){
			indices.push(i as u32);
		}
		
		println!("FRAMES: {:#?} TARGET: {:#?}", indices.len(), wlen);

		let img_path_len        = list[0].split("/").collect::<Vec<_>>().len(); 
		let frames : Vec<Frame> = Vec::new();
		let mframes             = Mutex::new(frames);

		list.par_iter().zip(indices).for_each(|(item, index)|{
			println!("ITEM {} INDEX {}", item, index);
			
			let in_path   = item.to_string(); 
		    let meta_data = parse_meta(in_path.split("/").collect::<Vec<_>>()[img_path_len - 1].to_string());

		    //colorize and add metadata as text to the frame
		    let img   = open_jp2(in_path.to_string());
		    let clut  = image::open(format!("media/colortables/{}_color_table.png", meta_data.wlen)).unwrap();
		    let img   = apply_clut(img, clut);
		    let frame = Frame{
		    	frm: img, 
		    	idx: index, 
		    	dat: meta_data
		    };

		    mframes.lock().unwrap().push(frame);

		});
		wlist.push(mframes);
	}

	let mut len = wlist[0].lock().unwrap().len();
	for wlen in wlist.iter(){
		//figure out which list is the shortest
		if wlen.lock().unwrap().len() < len {
			len = wlen.lock().unwrap().len();
		}
	}

	/*bring our colored frames out of the mutex. Is it necessary to do this? 
	Do I need to do this to sort and trim the lists? We put it all back in a mutex anyway.*/
	let mut cframes = Vec::new();
	for wlen in wlist.iter().rev(){
		let mut freed = sort(wlen.lock().unwrap().to_vec());
		freed.truncate(len); //trim all lists down to the lowest common number of frames. 
		cframes.push(freed);
	}
	
	//quick and dirty freeing up a bunch of RAM
	for _i in 0..wlist.len(){
		wlist.pop();
	}

	//store the start date of the video for the final filename
	let date      = &cframes[0][0].dat.date.replace("/", "_"); 
	let mut l_idx = 0;
	let mut wlist = Vec::new();
	
	//build our final composite frames
	for wlen in cframes.iter(){
		let frames : Vec<Frame> = Vec::new();
		let f_frames            = Mutex::new(frames);
		
		wlen.par_iter().for_each(|item|{
			println!("Compositing: : {} :: {}", manipulate::zfill(&item.idx.to_string(), 2), item.dat.wlen);

		    //build all the additional images to add to the frame
		    let mut frame = image::DynamicImage::new_rgb8(tp.resolution.0, tp.resolution.1);
		    let sun       = item.frm.resize(tp.resolution.1, tp.resolution.1, image::FilterType::Nearest);
		    let earth     = image::open(format!("media/misc/earth/earth_{}.png", item.dat.hour)).unwrap();
		    let gfx       = image::open(format!("media/misc/OVERLAY_{}_{}.png", tp.mus_id, l_idx)).unwrap();

			//add thumbnails of corresponding frames from other spectra
		    let thumb304  = cframes[0][item.idx as usize].frm.resize(tp.thumb_size, tp.thumb_size, image::FilterType::Nearest);
		    let thumb171  = cframes[1][item.idx as usize].frm.resize(tp.thumb_size, tp.thumb_size, image::FilterType::Nearest);
		    let thumb193  = cframes[2][item.idx as usize].frm.resize(tp.thumb_size, tp.thumb_size, image::FilterType::Nearest);
		    let thumb211  = cframes[3][item.idx as usize].frm.resize(tp.thumb_size, tp.thumb_size, image::FilterType::Nearest);
		    let thumb335  = cframes[4][item.idx as usize].frm.resize(tp.thumb_size, tp.thumb_size, image::FilterType::Nearest);
		    let thumb94   = cframes[5][item.idx as usize].frm.resize(tp.thumb_size, tp.thumb_size, image::FilterType::Nearest);

		    //add additional images to main frame
		    overlay(&mut frame, &sun,            tp.sun.0, tp.sun.1      );
		    overlay(&mut frame, &earth,        tp.earth.0, tp.earth.1    );
		    
		    overlay(&mut frame, &thumb94,    tp.thumb94.0, tp.thumb94.1  );
		    overlay(&mut frame, &thumb335,  tp.thumb335.0, tp.thumb335.1 );
		    overlay(&mut frame, &thumb211,  tp.thumb211.0, tp.thumb211.1 );
		    overlay(&mut frame, &thumb193,  tp.thumb193.0, tp.thumb193.1 );
		    overlay(&mut frame, &thumb171,  tp.thumb171.0, tp.thumb171.1 );
		    overlay(&mut frame, &thumb304,  tp.thumb304.0, tp.thumb304.1 );
		    
		    overlay(&mut frame, &gfx,        tp.overlay.0, tp.overlay.1  );


		    let frame = annotate(
		    	frame, 
		    	item.dat.date.to_string(), 
		    	tp.font.to_string(), 
		    	(tp.ts.0, tp.ts.1), 
		    	35.0, 
		    	(185u8, 185u8, 185u8, 0u8));
		    let frame = annotate(
		    	frame, 
		    	item.dat.time.to_string(), 
		    	tp.font.to_string(), 
		    	(tp.ts.0, tp.ts.1 + 35), 
		    	35.0, 
		    	(185u8, 185u8, 185u8, 0u8));
		    let frame = annotate(
		    	frame, 
		    	"Earth for scale".to_string(), 
		    	tp.font.to_string(), 
		    	(tp.earth.0 -2, tp.earth.1 + 65), 
		    	35.0, 
		    	(185u8, 185u8, 185u8, 0u8));

		    let output_frame = Frame{
		    	frm: frame,
		    	idx: item.idx,
		    	dat: MetaData{
		    		date: "test".to_string(),
		    		time: "test".to_string(),
		    		hour: "test".to_string(),
		    		wlen: item.dat.wlen.to_string()
		    	}
		    };
		    //add our processed frame to the tmp directory
		    f_frames.lock().unwrap().push(output_frame);
		});
		wlist.push(f_frames);
		l_idx += 1;
	}

	//freeing up RAM again, can we do this better?
	for _i in 0..cframes.len(){
		cframes.pop();
	}
 	
 	/* really not sure why I have to juggle this data so much to get it in the order I want, 
 	I'm pretty sure it worked before without this */
 	let mut prerender_frames = Vec::new();
	for wlen in wlist.iter(){
		let wlen = sort(wlen.lock().unwrap().to_vec()); 
		prerender_frames.push(wlen);
	}

	let mut final_out = Vec::new();
	for wlen in prerender_frames.iter(){
		for f in wlen.iter(){
			final_out.push(f);
		}
	}

	//so filenames are still in the correct order even though we parallelize the output.
	let mut indices : Vec<u32> = Vec::new(); 
	for i in 0..final_out.len(){ 
		indices.push(i as u32);
	}

	final_out.par_iter().zip(indices).for_each(|(frame, index)|{
		println!("prerendering: {}", index);
		frame.frm.save(format!("tmp/{}.png", manipulate::zfill(&index.to_string(), 4))).unwrap();
	});

	// build a video
	Command::new("sh")
		.arg("-c")
		.arg(format!("ffmpeg -r 24 -i tmp/%04d.png -vcodec libx264 -filter 'minterpolate=mi_mode=blend' -b:v 4M -pix_fmt yuv420p -y {}/{}_video.mp4", output_dir, date))
		.spawn()
		.expect("failed to execute process");
}
