use tagger::PathCommand;

use qrcodegen::QrCode;
use qrcodegen::QrCodeEcc;

use crate::blob::BlobObject;
use crate::chat::Chat;
use crate::chat::ChatId;
use crate::color::color_int_to_hex_string;
use crate::constants::DC_CONTACT_ID_SELF;
use crate::contact::Contact;
use crate::context::Context;
use crate::securejoin;
use crate::stock_str;
use anyhow::Result;

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

    inner_generate_secure_join_qr_code(
        &stock_str::verify_contact_qr_description(
            context,
            contact.get_display_name(), /* TODO fix that it doesn't say "Me" anymore */
            contact.get_addr(),
        )
        .await,
        &securejoin::dc_get_securejoin_qr(context, None).await?,
        &color_int_to_hex_string(contact.get_color()),
        avatar,
    )
}

fn inner_generate_secure_join_qr_code(
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
    let avatar_border_size = 8.0;

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
        for (count, line) in textwrap::fill(qrcode_description, max_text_width)
            .split('\n')
            .enumerate()
        {
            w.elem("text", |d| {
                d.attr("y", (count as f32 * text_font_size) + text_y_pos)
                    .attr("x", width / 2.0)
                    .attr("text-anchor", "middle")
                    .attr(
                        "style",
                        format!(
                            "font-family:monospace;\
                        font-style:normal;\
                        font-variant:normal;\
                        font-weight:normal;\
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

        const LOGO_SCALE: f32 = 0.68;
        const LOGO_SIZE: f32 = 118.0 * LOGO_SCALE;
        const HALF_LOGO_SIZE: f32 = LOGO_SIZE / 2.0;
        let logo_position_in_qr = (qr_code_size / 2.0) - HALF_LOGO_SIZE;
        let logo_position_x = ((width - qr_code_size) / 2.0) + logo_position_in_qr;
        let logo_position_y =
            ((height - qr_code_size) / 2.0) - qr_translate_up + logo_position_in_qr;

        if let Some(img) = avatar {
            //convert image to base64
            let png_data = base64::encode(img);

            w.single("circle", |d| {
                d.attr("cx", logo_position_x + HALF_LOGO_SIZE)
                    .attr("cy", logo_position_y + HALF_LOGO_SIZE)
                    .attr("r", HALF_LOGO_SIZE + avatar_border_size)
                    .attr("style", "fill:white");
            });

            w.single("image", |d| {
                d.attr("x", logo_position_x)
                    .attr("y", logo_position_y)
                    .attr("width", HALF_LOGO_SIZE * 2.0)
                    .attr("height", HALF_LOGO_SIZE * 2.0)
                    .attr("preserveAspectRatio", "none")
                    .attr("clip-path", format!("circle({}px)", HALF_LOGO_SIZE))
                    // might need xlink:href instead if it doesn't work on older devices??
                    .attr("href", format!("data:image/jpeg;base64,{}", png_data));
            });
        } else {
            w.elem("g", |d| {
                d.attr(
                    "transform",
                    format!(
                        "translate({},{}),scale({})",
                        logo_position_x, logo_position_y, LOGO_SCALE
                    ),
                );
            })
            .build(|w| {
                w.put_raw(format!(
                    include_str!("../assets/qrcode_logo_avatar.svg"),
                    color
                ));
            });
        }
    });

    Ok(svg)
}
