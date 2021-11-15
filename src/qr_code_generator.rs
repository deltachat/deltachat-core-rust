use tagger::PathCommand;

use qrcodegen::QrCode;
use qrcodegen::QrCodeEcc;

use crate::blob::BlobObject;
use crate::color::color_int_to_hex_string;
use crate::constants::DC_CONTACT_ID_SELF;
use crate::contact::Contact;
use crate::context::Context;
use crate::securejoin;
use crate::stock_str;
use anyhow::Result;

// pub fn generate_join_group_qr_code(context: &Context, qr_code_content) -> String {

// }

pub async fn generate_verification_qr(context: &Context) -> Result<String> {
    let contact = Contact::get_by_id(context, DC_CONTACT_ID_SELF).await?;

    let avatar = match contact.get_profile_image(context).await? {
        Some(path) => {
            let avatar_blob = BlobObject::from_path(context, &path)?;
            Some(std::fs::read(avatar_blob.to_abs_path())?)
        }
        None => None,
    };

    inner_generate_secure_join_qr_code(
        context,
        &stock_str::verify_contact_qr_description(context, &contact).await,
        &securejoin::dc_get_securejoin_qr(context, None).await?,
        &color_int_to_hex_string(contact.get_color()),
        avatar,
    )
}

fn inner_generate_secure_join_qr_code(
    context: &Context,
    qrcode_description: &str,
    qrcode_content: &str,
    color: &str,
    avatar: Option<Vec<u8>>,
) -> Result<String> {
    // config
    let width = 560.0;
    let height = 630.0;
    let corner_boldness = 20.0;
    let corner_padding = 20.0;
    let corner_height = (height - (corner_padding * 2.0)) / 7.0;
    let corner_width = (width - (corner_padding * 2.0)) / 5.0;
    let qr_code_size = 360.0;
    let qr_translate_up = 40.0;
    let text_y_pos = ((height - qr_code_size) / 2.0) + qr_code_size;
    let text_font_size = 22.0;
    let max_text_width = 34;
    let avatar_border_size = 10.0;

    let qr = QrCode::encode_text(qrcode_content, QrCodeEcc::High).unwrap(); //todo replace this unwrap with ?

    let mut svg = String::with_capacity(28000);

    let mut w = tagger::new(&mut svg);

    w.elem("svg", |d| {
        d.attr("xmlns", "http://www.w3.org/2000/svg")
            .attr("viewBox", format_args!("0 0 {} {}", width, height));
    })
    .build(|w| {
        // White Background
        w.single("rect", |d| {
            d.attr("x", 0)
                .attr("y", 0)
                .attr("width", width)
                .attr("height", height)
                .attr("style", "fill:white");
        });
        // Corners
        {
            let border_style = format!("fill:{}", color);
            // upper, left corner
            w.single("path", |d| {
                d.attr("x", 0)
                    .attr("y", 0)
                    .attr("style", &border_style)
                    .path(|p| {
                        p.put(PathCommand::M(corner_padding, corner_padding));
                        p.put(PathCommand::V(corner_width + corner_padding));
                        p.put(PathCommand::H(corner_boldness + corner_padding));
                        p.put(PathCommand::V(corner_boldness + corner_padding));
                        p.put(PathCommand::H(corner_height + corner_padding));
                        p.put(PathCommand::V(corner_padding));
                        p.put(PathCommand::Z(corner_padding));
                    });
            });
            // upper, right corner
            w.single("path", |d| {
                d.attr("x", 0)
                    .attr("y", 0)
                    .attr("style", &border_style)
                    .path(|p| {
                        p.put(PathCommand::M(width - corner_padding, corner_padding));
                        p.put(PathCommand::V(corner_width + corner_padding));
                        p.put(PathCommand::H(width - (corner_boldness + corner_padding)));
                        p.put(PathCommand::V(corner_boldness + corner_padding));
                        p.put(PathCommand::H(width - (corner_height + corner_padding)));
                        p.put(PathCommand::V(corner_padding));
                        p.put(PathCommand::Z(0));
                    });
            });
            // lower, right corner
            w.single("path", |d| {
                d.attr("x", 0)
                    .attr("y", 0)
                    .attr("style", &border_style)
                    .path(|p| {
                        p.put(PathCommand::M(
                            width - corner_padding,
                            height - corner_padding,
                        ));
                        p.put(PathCommand::V(height - (corner_width + corner_padding)));
                        p.put(PathCommand::H(width - (corner_boldness + corner_padding)));
                        p.put(PathCommand::V(height - (corner_boldness + corner_padding)));
                        p.put(PathCommand::H(width - (corner_height + corner_padding)));
                        p.put(PathCommand::V(height - corner_padding));
                        p.put(PathCommand::Z(0));
                    });
            });
            // lower, left corner
            w.single("path", |d| {
                d.attr("x", 0)
                    .attr("y", 0)
                    .attr("style", &border_style)
                    .path(|p| {
                        p.put(PathCommand::M(corner_padding, height - corner_padding));
                        p.put(PathCommand::V(height - (corner_width + corner_padding)));
                        p.put(PathCommand::H(corner_boldness + corner_padding));
                        p.put(PathCommand::V(height - (corner_boldness + corner_padding)));
                        p.put(PathCommand::H(corner_height + corner_padding));
                        p.put(PathCommand::V(height - corner_padding));
                        p.put(PathCommand::Z(0));
                    });
            });
        }
        // Qrcode Background
        w.single("rect", |d| {
            d.attr("x", (width - qr_code_size) / 2.0)
                .attr("y", ((height - qr_code_size) / 2.0) - qr_translate_up)
                .attr("width", qr_code_size)
                .attr("height", qr_code_size)
                .attr("style", "fill:white");
        });
        // Qrcode
        w.elem("g", |d| {
            d.attr(
                "transform",
                format!(
                    "translate({},{})",
                    (width - qr_code_size) / 2.0,
                    ((height - qr_code_size) / 2.0) - qr_translate_up
                ),
            );
            // If the qr code should be in the wrong place,
            // we could also translate and scale the points in the path already,
            // but that would make the resulting svg way bigger in size and might bring up rounding issues,
            // so better avoid doing it manually if possible
        })
        .build(|w| {
            w.single("path", |d| {
                let mut path_data = String::with_capacity(0);
                let scale = qr_code_size / qr.size() as f32;

                for y in 0..qr.size() {
                    for x in 0..qr.size() {
                        if qr.get_module(x, y) {
                            path_data += &format!("M{},{}h1v1h-1z", x, y);
                        }
                    }
                }

                d.attr("style", "fill:#000000")
                    .attr("d", path_data)
                    .attr("transform", format!("scale({})", scale));
            });
        });
        // Text
        for (count, line) in textwrap::fill(qrcode_description, max_text_width).split('\n').enumerate() {
            w.elem("text", |d| {
                d.attr("y", (count as f32 * text_font_size) + text_y_pos)
                .attr("x", width/2.0)
                .attr("text-anchor", "middle")
                .attr("style", format!("font-family:monospace;font-style:normal;font-variant:normal;font-weight:normal;font-stretch:normal;font-size:{}px;font-variant-caps:normal;font-variant-numeric:normal;font-variant-east-asian:normal;fill:#000000;fill-opacity:1;stroke:none", text_font_size));
            })
            .build(|w|{w.put_raw(line);});
        }
        // Logo / contact image in middle of qrcode

        const HALF_LOGO_SIZE:f32 = 118.0 / 2.0;
        let logo_position_in_qr = (qr_code_size/ 2.0) - HALF_LOGO_SIZE;
        let logo_position_x = ((width - qr_code_size) / 2.0) + logo_position_in_qr;
        let logo_position_y = ((height - qr_code_size) / 2.0) - qr_translate_up + logo_position_in_qr;

        if let Some(img) = avatar {


            let png_data = {
            //     let mut bytes: Vec<u8> = Vec::new();
            // img.write_to(&mut bytes, image::ImageOutputFormat::Jpeg(255)).unwrap(); // TODO replace with ?
            // make image square
            // make image round
            // convert image to base64
            base64::encode(img)};

            w.single("circle", |d|{
                d.attr("cx", logo_position_x+HALF_LOGO_SIZE)
                .attr("cy", logo_position_y+HALF_LOGO_SIZE)
                .attr("r", HALF_LOGO_SIZE + avatar_border_size)
                .attr("style", "fill:white");
            });

            w.single("image", |d|{
                d.attr("x", logo_position_x)
                .attr("y", logo_position_y)
                .attr("width", HALF_LOGO_SIZE*2.0)
                .attr("height", HALF_LOGO_SIZE*2.0)
                .attr("preserveAspectRatio", "none")
                .attr("clip-path", format!("circle({}px)", HALF_LOGO_SIZE))
                .attr("href", format!("data:image/jpeg;base64,{}", png_data)); // might need xlink:href instead if it doesn't work on older devices??
            });

        } else {
            w.elem("g", |d|{
                d.attr("transform",format!("translate({},{})", logo_position_x, logo_position_y));
            }).build(|w|{
                w.put_raw(format!(r#"<rect
                style="fill:#ffffff"
                id="rect1126"
                width="118"
                height="118"
                x="0"
                y="0"
                ry="28.223997" />
             <path
                id="path3799"
                style="fill:{};fill-opacity:1;stroke:none;stroke-width:0.841384"
                d="M 59.01639,16.20193 C 35.3978,16.48935 16.24055,35.89659 16.24055,59.5365 c 0,23.63994 19.15725,42.58177 42.77584,42.29435 22.45621,-0.0903 17.17604,-12.54348 41.99855,-1.01229 -13.5976,-21.24148 0.46063,-24.06917 0.77729,-42.32228 0,-23.63993 -19.15724,-42.58176 -42.77584,-42.29435 z m -0.1479,12.47451 c 4.59808,0 8.60394,0.63421 12.01766,1.90133 3.44856,1.26713 5.1732,3.01277 5.1732,5.23729 0,1.07001 -0.41785,1.95658 -1.25385,2.66055 -0.83603,0.70395 -1.81209,1.05665 -2.92678,1.05665 -1.60235,0 -3.48349,-0.97233 -5.64319,-2.91526 -2.19454,-1.97109 -4.05791,-3.35111 -5.5906,-4.13954 -1.49786,-0.81659 -3.25637,-1.22428 -5.27673,-1.22428 -2.57771,0 -4.70408,0.46431 -6.37611,1.39354 -1.6372,0.92923 -2.45514,2.11187 -2.45514,3.54795 0,1.3516 0.6792,2.62001 2.03773,3.80266 1.35852,1.18265 4.85942,3.33599 10.50252,6.46157 6.02626,3.35084 10.27573,5.97014 12.74894,7.85675 2.50804,1.88661 4.54566,4.1807 6.11318,6.8839 1.56753,2.7032 2.35161,5.56195 2.35161,8.57489 0,5.29378 -2.31674,9.96772 -6.94965,14.02252 -4.59808,4.02665 -9.9801,6.03923 -16.1457,6.03923 -5.60826,0 -10.34494,-1.61782 -14.21151,-4.85603 -3.86656,-3.23821 -5.7993,-7.5611 -5.7993,-12.96751 0,-5.2093 2.12474,-9.55968 6.37447,-13.05131 4.28457,-3.49164 9.54397,-5.60291 15.77924,-6.33504 -1.74169,-1.57686 -4.16329,-3.4649 -7.26351,-5.66126 -3.41372,-2.42162 -5.7127,-4.32095 -6.89705,-5.7007 -1.18435,-1.40793 -1.77644,-2.94327 -1.77644,-4.60461 0,-2.47794 1.42942,-4.42097 4.28581,-5.82889 2.85638,-1.43607 6.58313,-2.1544 11.1812,-2.1544 z m 2.76901,25.89228 c -9.99734,1.32345 -14.99537,6.87103 -14.99537,16.64199 0,5.04035 1.23578,8.95391 3.70899,11.74158 2.50805,2.78768 5.41685,4.18063 8.72608,4.18063 3.44854,0 6.28799,-1.33632 8.51736,-4.01136 2.22938,-2.70321 3.34418,-6.34983 3.34418,-10.93964 0,-6.64537 -3.1008,-12.51653 -9.30124,-17.6132 z" />"#, color));
            });
        }
    });

    Ok(svg)
}
