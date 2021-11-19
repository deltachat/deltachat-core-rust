use anyhow::Result;
use qrcodegen::{QrCode, QrCodeEcc};
use tagger::PathCommand;

use crate::{
    blob::BlobObject,
    chat::{Chat, ChatId},
    color::color_int_to_hex_string,
    config::Config,
    constants::DC_CONTACT_ID_SELF,
    contact::Contact,
    context::Context,
    securejoin, stock_str,
};

pub async fn get_securejoin_qr_svg(context: &Context, chat_id: Option<ChatId>) -> Result<String> {
    if let Some(chat_id) = chat_id {
        generate_join_group_qr_code(context, chat_id).await
    } else {
        generate_verification_qr(context).await
    }
}

async fn generate_join_group_qr_code(context: &Context, chat_id: ChatId) -> Result<String> {
    let chat = Chat::load_from_db(context, chat_id).await?;

    let avatar = match chat.get_profile_image(context).await? {
        Some(path) => {
            let avatar_blob = BlobObject::from_path(context, &path)?;
            Some(std::fs::read(avatar_blob.to_abs_path())?)
        }
        None => None,
    };

    inner_generate_secure_join_qr_code(
        &stock_str::secure_join_group_qr_description(context, &chat).await,
        &securejoin::dc_get_securejoin_qr(context, Some(chat_id)).await?,
        &color_int_to_hex_string(chat.get_color(context).await?),
        avatar,
        chat.get_name().chars().next().unwrap_or('#'),
    )
}

async fn generate_verification_qr(context: &Context) -> Result<String> {
    let contact = Contact::get_by_id(context, DC_CONTACT_ID_SELF).await?;

    let avatar = match contact.get_profile_image(context).await? {
        Some(path) => {
            let avatar_blob = BlobObject::from_path(context, &path)?;
            Some(std::fs::read(avatar_blob.to_abs_path())?)
        }
        None => None,
    };

    let displayname = match context.get_config(Config::Displayname).await? {
        Some(name) => name,
        None => contact.get_addr().to_owned(),
    };

    inner_generate_secure_join_qr_code(
        &stock_str::verify_contact_qr_description(context, &displayname, contact.get_addr()).await,
        &securejoin::dc_get_securejoin_qr(context, None).await?,
        &color_int_to_hex_string(contact.get_color()),
        avatar,
        displayname.chars().next().unwrap_or('#'),
    )
}

fn inner_generate_secure_join_qr_code(
    raw_qrcode_description: &str,
    qrcode_content: &str,
    color: &str,
    avatar: Option<Vec<u8>>,
    avatar_letter: char,
) -> Result<String> {
    // config
    let width = 560.0;
    let qrcode_description = &simple_escape_xml(raw_qrcode_description);
    let height = 630.0;
    let corner_boldness = 18.0;
    let corner_padding = 20.0;
    let corner_length = (width - (corner_padding * 2.0)) / 5.0;
    let qr_code_size = 400.0;
    let qr_translate_up = 40.0;
    let text_y_pos = ((height - qr_code_size) / 2.0) + qr_code_size;
    let (text_font_size, max_text_width) = if qrcode_description.len() <= 75 {
        (27.0, 32)
    } else {
        (19.0, 38)
    };
    let avatar_border_size = 9.0;

    let qr = QrCode::encode_text(qrcode_content, QrCodeEcc::Medium)?;
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
            let border_style = "fill:#2090ea";
            // upper, left corner
            w.single("path", |d| {
                d.attr("x", 0)
                    .attr("y", 0)
                    .attr("style", &border_style)
                    .path(|p| {
                        p.put(PathCommand::M(corner_padding, corner_padding));
                        p.put(PathCommand::V(corner_length + corner_padding));
                        p.put(PathCommand::H(corner_boldness + corner_padding));
                        p.put(PathCommand::V(corner_boldness + corner_padding));
                        p.put(PathCommand::H(corner_length + corner_padding));
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
                        p.put(PathCommand::V(corner_length + corner_padding));
                        p.put(PathCommand::H(width - (corner_boldness + corner_padding)));
                        p.put(PathCommand::V(corner_boldness + corner_padding));
                        p.put(PathCommand::H(width - (corner_length + corner_padding)));
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
                        p.put(PathCommand::V(height - (corner_length + corner_padding)));
                        p.put(PathCommand::H(width - (corner_boldness + corner_padding)));
                        p.put(PathCommand::V(height - (corner_boldness + corner_padding)));
                        p.put(PathCommand::H(width - (corner_length + corner_padding)));
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
                        p.put(PathCommand::V(height - (corner_length + corner_padding)));
                        p.put(PathCommand::H(corner_boldness + corner_padding));
                        p.put(PathCommand::V(height - (corner_boldness + corner_padding)));
                        p.put(PathCommand::H(corner_length + corner_padding));
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
        for (count, line) in textwrap::fill(qrcode_description, max_text_width)
            .split('\n')
            .enumerate()
        {
            w.elem("text", |d| {
                d.attr("y", (count as f32 * (text_font_size * 1.2)) + text_y_pos)
                    .attr("x", width / 2.0)
                    .attr("text-anchor", "middle")
                    .attr("font-weight", "bold")
                    .attr(
                        "style",
                        format!(
                            "font-family:Arial, sans-serif;\
                        font-style:normal;\
                        font-stretch:normal;\
                        font-size:{}px;\
                        fill:#000000;\
                        stroke:none",
                            text_font_size
                        ),
                    );
            })
            .build(|w| {
                w.put_raw(line);
            });
        }
        // Logo / contact image in middle of qrcode
        const LOGO_SIZE: f32 = 94.4;
        const HALF_LOGO_SIZE: f32 = LOGO_SIZE / 2.0;
        let logo_position_in_qr = (qr_code_size / 2.0) - HALF_LOGO_SIZE;
        let logo_position_x = ((width - qr_code_size) / 2.0) + logo_position_in_qr;
        let logo_position_y =
            ((height - qr_code_size) / 2.0) - qr_translate_up + logo_position_in_qr;

        w.single("circle", |d| {
            d.attr("cx", logo_position_x + HALF_LOGO_SIZE)
                .attr("cy", logo_position_y + HALF_LOGO_SIZE)
                .attr("r", HALF_LOGO_SIZE + avatar_border_size)
                .attr("style", "fill:white");
        });

        if let Some(img) = avatar {
            w.elem("defs", |_| {}).build(|w| {
                w.elem("clipPath", |d| {
                    d.attr("id", "avatar-cut");
                })
                .build(|w| {
                    w.single("circle", |d| {
                        d.attr("cx", logo_position_x + HALF_LOGO_SIZE)
                            .attr("cy", logo_position_y + HALF_LOGO_SIZE)
                            .attr("r", HALF_LOGO_SIZE);
                    });
                });
            });

            w.single("image", |d| {
                d.attr("x", logo_position_x)
                    .attr("y", logo_position_y)
                    .attr("width", HALF_LOGO_SIZE * 2.0)
                    .attr("height", HALF_LOGO_SIZE * 2.0)
                    .attr("preserveAspectRatio", "none")
                    .attr("clip-path", "url(#avatar-cut)")
                    .attr(
                        "href" /*might need xlink:href instead if it doesn't work on older devices?*/, 
                        format!("data:image/jpeg;base64,{}", base64::encode(img)),
                    );
            });
        } else {
            w.single("circle", |d| {
                d.attr("cx", logo_position_x + HALF_LOGO_SIZE)
                    .attr("cy", logo_position_y + HALF_LOGO_SIZE)
                    .attr("r", HALF_LOGO_SIZE)
                    .attr("style", format!("fill:{}", &color));
            });

            let avatar_font_size = LOGO_SIZE * 0.65;
            let font_offset = LOGO_SIZE * 0.35 / 2.0 * 1.3;
            w.elem("text", |d| {
                d.attr("y", logo_position_y + HALF_LOGO_SIZE + font_offset)
                    .attr("x", logo_position_x + HALF_LOGO_SIZE)
                    .attr("text-anchor", "middle")
                    .attr("font-weight", "400")
                    .attr(
                        "style",
                        format!(
                            "font-family:sans-serif;font-size:{}px;fill:#ffffff;",
                            avatar_font_size
                        ),
                    );
            })
            .build(|w| {
                w.put_raw(avatar_letter.to_uppercase());
            });
        }
    });

    Ok(svg)
}

fn simple_escape_xml(xml: &str) -> String {
    // this escaping method copies the data 5 times?
    xml.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('\'', "&apos;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod text_simple_escape_xml {
    use super::simple_escape_xml;
    #[test]
    fn test() {
        assert_eq!(
            &simple_escape_xml("<circle r=\"6px\"></circle>&'test'"),
            "&lt;circle r=&quot6px&quot&gt;&lt;/circle&gt;&amp;&apostest&apos"
        )
    }
}
