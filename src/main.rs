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

use imageproc::drawing::draw_text_mut;
use image::{GenericImage, GenericImageView, Rgba};
use image::imageops::overlay;

use rusttype::{FontCollection, Scale};
use glob::glob;
use rayon::prelude::*;
use std::env;
use voca_rs::*;
use std::process::Command;

struct MetaData{
	date: String,
	time: String,
	wlen: String
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

fn annotate(mut frame: image::DynamicImage, metadata: MetaData) -> image::DynamicImage{
	let font   = Vec::from(include_bytes!("BebasNeue-Regular.ttf") as &[u8]);
    let font   = FontCollection::from_bytes(font).unwrap().into_font().unwrap();
    let height = 60.0;
    let scale  = Scale { x: height * 2.0, y: height };

    draw_text_mut(&mut frame, Rgba([255u8, 255u8, 255u8, 0u8]), 3062, 310, scale, &font, &metadata.date);
    draw_text_mut(&mut frame, Rgba([255u8, 255u8, 255u8, 0u8]), 3062, 310 + height as u32, scale, &font, &metadata.time);

	return frame;
}

fn main(){
	let args: Vec<String> = env::args().collect();
	let target_dir = &args[1];
	let output_dir = &args[2];
	let wavlist    = ["94", "335", "211", "193", "171", "304"];

	for wlen in wavlist.iter(){
		let list = build_list(target_dir.to_string() + "/" + wlen);
		let mut indices : Vec<i16> = Vec::new();
		
		for i in 0..list.len(){
			indices.push(i as i16);
		}
		println!("FRAMES: {:#?} TARGET: {:#?} OUTPUT: {:#?}", indices.len(), target_dir, output_dir);

		let img_path_len = list[0].split("/").collect::<Vec<_>>().len(); 

		list.par_iter().zip(indices).for_each(|(item, index)|{
			println!("ITEM {} INDEX {}", item, index);
			let in_path   = item.to_string(); 
		    let meta_data = parse_meta(in_path.split("/").collect::<Vec<_>>()[img_path_len - 1].to_string());
		    let out_path  = "tmp/".to_owned() + &meta_data.wlen.to_owned() + "/colored/" + &index.to_string() + ".png" ;

		    //colorize and add metadata as text to the frame
		    let img  = open_jp2(in_path.to_string());
		    let clut = image::open("media/colortables/".to_owned() + &meta_data.wlen + "_color_table.png").unwrap();

		    let img = apply_clut(img, clut);
		    let img = annotate(img, meta_data);

		    img.save(out_path).unwrap();

		});
	}

	//Take a second pass to build out our finalized frames with colored images- this requires a second iteration through all the colored frames because each final frame needs a thumbnail from all the other spectra
	
	for (idx, wlen) in wavlist.iter().enumerate(){

		let mut list : Vec<String> = vec!("init".to_string());
		let globtarget = &("tmp/".to_owned() + &(wlen.to_owned().to_owned() + "/colored/*.png"));
		println!("GLOBTARGET: {}", globtarget);
		list.pop();
		for entry in glob(globtarget).expect("Failed to read glob pattern"){
			match entry{
				Ok(path) => list.push(path.display().to_string()),
				Err(e)   => println!("{:?}", e),
			}
		}

		let mut indices : Vec<i16> = Vec::new();
		
		for i in 0..list.len(){
			indices.push(i as i16);
		}
		println!("FINISHING: {:#?} TARGET: {:#?}", indices.len(), wlen);
		list.par_iter().zip(indices).for_each(|(item, index)|{
			
			let out_path = "tmp/".to_owned() + &wlen.to_owned() + "/finished/" + &manipulate::zfill(&index.to_string(), 4) + ".png";
			

		    //build all the additional images to add to the frame
		    // let mut frame = image::open("media/misc/TEMPLATE_2x3.png").unwrap();
		    let mut frame = image::DynamicImage::new_rgb8(3840, 3240);
		    let sun       = image::open(item.to_string()).unwrap().resize(3240, 3240, image::FilterType::Nearest);
		    let earth     = image::open("media/misc/earth.png").unwrap();
		    let gfx       = image::open(format!("media/misc/OVERLAY_2x3_WHITE_{}.png", idx)).unwrap();
		    let thumb94   = image::open("tmp/94/colored/".to_owned() + &index.to_string() + ".png").unwrap().resize(180, 180, image::FilterType::Nearest);
		    let thumb304  = image::open("tmp/304/colored/".to_owned() + &index.to_string() + ".png").unwrap().resize(180, 180, image::FilterType::Nearest);
		    let thumb171  = image::open("tmp/171/colored/".to_owned() + &index.to_string() + ".png").unwrap().resize(180, 180, image::FilterType::Nearest);
		    let thumb193  = image::open("tmp/193/colored/".to_owned() + &index.to_string() + ".png").unwrap().resize(180, 180, image::FilterType::Nearest);
		    let thumb211  = image::open("tmp/211/colored/".to_owned() + &index.to_string() + ".png").unwrap().resize(180, 180, image::FilterType::Nearest);
		    let thumb335  = image::open("tmp/335/colored/".to_owned() + &index.to_string() + ".png").unwrap().resize(180, 180, image::FilterType::Nearest);

		    println!("ADDING OVERLAY: {}", out_path);

		    //add additional images to main frame
		    overlay(&mut frame, &sun,      345, 0 );
		    overlay(&mut frame, &earth,    250, 2750);
		    overlay(&mut frame, &thumb94,   88, 650 );
		    overlay(&mut frame, &thumb335,  88, 875 );
		    overlay(&mut frame, &thumb211,  88, 1105);
		    overlay(&mut frame, &thumb193,  88, 1335);
		    overlay(&mut frame, &thumb171,  88, 1560);
		    overlay(&mut frame, &thumb304,  88, 1790);
		    overlay(&mut frame, &gfx,        0, 0   );

		    //add our processed frame to the tmp directory
		    frame.save(out_path).unwrap();
		});
	}

	// build the final video
	let oname = "output_video.mp4";
	let mut flist : Vec<String> = Vec::new();

	for wlen in wavlist.iter().rev(){
		for entry in glob(&format!("tmp/{}/finished/*.png", wlen).to_string()).expect("Failed to read glob pattern"){
			match entry{
				Ok(path) => flist.push(path.display().to_string()),
				Err(e)   => println!("{:?}", e),
			}
		}
	}
	
	for (i, f) in flist.iter().enumerate(){
		let o = format!("tmp/ff/{}.png", manipulate::zfill(&i.to_string(), 4));
		Command::new("sh")
			.arg("-c")
			.arg(format!("cp {} {}", f, o))
			.output()
			.expect("failed to execute process");

	}

	Command::new("sh")
		.arg("-c")
		.arg(format!("ffmpeg -r 24 -i tmp/ff/%04d.png -vcodec libx264 -filter 'minterpolate=mi_mode=blend' -b:v 4M -pix_fmt yuv420p  -y {}/{}", output_dir, oname))
		.spawn()
		.expect("failed to execute process");

}
