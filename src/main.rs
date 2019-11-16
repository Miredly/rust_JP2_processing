extern crate image;
extern crate imageproc;
extern crate palette;
extern crate rusttype;
extern crate glob;
extern crate jpeg2000;
extern crate rayon;
extern crate kiss3d; 
extern crate nalgebra as na;
extern crate voca_rs;

use std::process::Command;
use std::sync::{Mutex};
use std::env;
use imageproc::drawing::draw_text_mut;
use image::{GenericImage, GenericImageView, Rgba};
use image::imageops::overlay;
use rusttype::{FontCollection, Scale};
use glob::glob;
use rayon::prelude::*;
use voca_rs::*;

#[derive(Clone)]
struct MetaData{
	date: String,
	time: String,
	wlen: String
}

#[derive(Clone)]
struct Frame {
	frm: image::DynamicImage,
	idx: i16,
	dat: MetaData
}

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

fn sort(mut flist : Vec<Frame>) -> Vec<Frame>{
	println!("SORTING...");
	flist.sort_by(|a, b| b.idx.cmp(&a.idx));
	return flist;
}

fn parse_meta(file: String) -> MetaData {

	let split = file.split("__").collect::<Vec<_>>();
	let data  = MetaData{
		date: split[0].to_string().replace("_", "/"),
		time: split[1].to_string().replace("_", ":"),
		wlen: split[2].split("_").collect::<Vec<_>>()[3].split(".").collect::<Vec<_>>()[0].to_string()
	};
	return data
}

fn open_jp2(path: String) -> image::DynamicImage{
    let codec      = jpeg2000::decode::Codec::JP2;
    let colorspace = jpeg2000::decode::DecodeConfig{ default_colorspace: Some(jpeg2000::decode::ColorSpace::SRGB), discard_level: 0};
    return           jpeg2000::decode::from_file(path, codec, colorspace, None).unwrap();
}

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

fn annotate(mut frame: image::DynamicImage, metadata: &MetaData) -> image::DynamicImage{
	let font   = Vec::from(include_bytes!("BebasNeue-Regular.ttf") as &[u8]);
    let font   = FontCollection::from_bytes(font).unwrap().into_font().unwrap();
    let height = 35.0;
    let scale  = Scale { x: height * 2.0, y: height };
    //timestamp
    draw_text_mut(&mut frame, Rgba([185u8, 185u8, 185u8, 0u8]), 3062, 312, scale, &font, &metadata.date);
    draw_text_mut(&mut frame, Rgba([185u8, 185u8, 185u8, 0u8]), 3062, 312 + height as u32, scale, &font, &metadata.time);
    //"earth for scale"
    draw_text_mut(&mut frame, Rgba([185u8, 185u8, 185u8, 0u8]),  500, 2800, scale, &font, "earth for size scale");

	return frame;
}

fn main(){
	//clear tmp
	Command::new("sh")
		.arg("-c")
		.arg(format!("rm -r tmp/ff/*.png"))
		.spawn()
		.expect("failed to execute process");

	let args: Vec<String> = env::args().collect();
	let target_dir        = &args[1];
	let output_dir        = &args[2];
	let wavlist           = ["94", "335", "211", "193", "171", "304"];
	let mut wlist         = Vec::new(); 

	for wlen in wavlist.iter(){
		let list = build_list(target_dir.to_string() + "/" + wlen);
		let mut indices : Vec<i16> = Vec::new();
		
		for i in 0..list.len(){
			indices.push(i as i16);
		}
		println!("FRAMES: {:#?} TARGET: {:#?} OUTPUT: {:#?}", indices.len(), target_dir, output_dir);

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

	//bring our colored frames out of the mutex
	let mut cframes = Vec::new();
	for wlen in wlist.iter().rev(){
		let freed = sort(wlen.lock().unwrap().to_vec());
		cframes.push(freed);
	}

	let frames : Vec<image::DynamicImage> = Vec::new();
	let f_frames                          = Mutex::new(frames);
	let mut l_idx                         = 0;
	
	//build our final composite frames
	for wlen in cframes.iter(){
		
		wlen.par_iter().for_each(|item|{
			println!("CHECK: : {} :: {}", manipulate::zfill(&item.idx.to_string(), 2), item.dat.wlen);
		    //build all the additional images to add to the frame
		    let mut frame = image::DynamicImage::new_rgb8(3840, 3240);
		    let sun       = item.frm.resize(3240, 3240, image::FilterType::Nearest);
		    let earth     = image::open("media/misc/earth.png").unwrap();
		    let gfx       = image::open(format!("media/misc/OVERLAY_2x3_WHITE_{}.png", l_idx)).unwrap();
		    let thumb304  = cframes[0][item.idx as usize].frm.resize(180, 180, image::FilterType::Nearest);
		    let thumb171  = cframes[1][item.idx as usize].frm.resize(180, 180, image::FilterType::Nearest);
		    let thumb193  = cframes[2][item.idx as usize].frm.resize(180, 180, image::FilterType::Nearest);
		    let thumb211  = cframes[3][item.idx as usize].frm.resize(180, 180, image::FilterType::Nearest);
		    let thumb335  = cframes[4][item.idx as usize].frm.resize(180, 180, image::FilterType::Nearest);
		    let thumb94   = cframes[5][item.idx as usize].frm.resize(180, 180, image::FilterType::Nearest);

		    //add additional images to main frame
		    overlay(&mut frame, &sun,      345, 0    );
		    overlay(&mut frame, &earth,    500, 2750 );
		    overlay(&mut frame, &thumb94,   88, 650  );
		    overlay(&mut frame, &thumb335,  88, 875  );
		    overlay(&mut frame, &thumb211,  88, 1105 );
		    overlay(&mut frame, &thumb193,  88, 1335 );
		    overlay(&mut frame, &thumb171,  88, 1560 );
		    overlay(&mut frame, &thumb304,  88, 1790 );
		    overlay(&mut frame, &gfx,        0, 0    );

		    let frame = annotate(frame, &item.dat);

		    //add our processed frame to the tmp directory
		    f_frames.lock().unwrap().push(frame);
		});
		l_idx += 1;
	}

	for (idx, frame) in f_frames.lock().unwrap().iter().enumerate(){
		frame.save(format!("tmp/ff/{}.png", manipulate::zfill(&idx.to_string(), 4))).unwrap();
	}

	// build a video
	Command::new("sh")
		.arg("-c")
		.arg(format!("ffmpeg -r 24 -i tmp/ff/%04d.png -vcodec libx264 -filter 'minterpolate=mi_mode=blend' -b:v 4M -pix_fmt yuv420p  -y {}/{}", output_dir, "testvideo.mp4"))
		.spawn()
		.expect("failed to execute process");
}
