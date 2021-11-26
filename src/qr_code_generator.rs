use anyhow::Result;
use qrcodegen::{QrCode, QrCodeEcc};

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
        &stock_str::setup_contact_qr_description(context, &displayname, contact.get_addr()).await,
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
    let qrcode_description = &escaper::encode_minimal(raw_qrcode_description);
    // config
    let width = 515.0;
    let height = 630.0;
    let logo_offset = 28.0;
    let qr_code_size = 400.0;
    let qr_translate_up = 40.0;
    let text_y_pos = ((height - qr_code_size) / 2.0) + qr_code_size;
    let avatar_border_size = 9.0;
    let card_border_size = 2.0;
    let card_roundness = 40.0;

    let qr = QrCode::encode_text(qrcode_content, QrCodeEcc::Medium)?;
    let mut svg = String::with_capacity(28000);
    let mut w = tagger::new(&mut svg);

    w.elem("svg", |d| {
        d.attr("xmlns", "http://www.w3.org/2000/svg")
            .attr("viewBox", format_args!("0 0 {} {}", width, height));
    })
    .build(|w| {
        // White Background apears like a card
        w.single("rect", |d| {
            d.attr("x", card_border_size)
                .attr("y", card_border_size)
                .attr("rx", card_roundness)
                .attr("stroke", "#c6c6c6")
                .attr("stroke-width", card_border_size)
                .attr("width", width - (card_border_size * 2.0))
                .attr("height", height - (card_border_size * 2.0))
                .attr("style", "fill:#f2f2f2");
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
        const BIG_TEXT_CHARS_PER_LINE: usize = 32;
        const SMALL_TEXT_CHARS_PER_LINE: usize = 38;
        let chars_per_line = if qrcode_description.len() > SMALL_TEXT_CHARS_PER_LINE*2 {
            SMALL_TEXT_CHARS_PER_LINE
        } else {
            BIG_TEXT_CHARS_PER_LINE
        };
        let lines = textwrap::fill(qrcode_description, chars_per_line);
        let (text_font_size, text_y_shift) = if lines.split('\n').count() <= 2 {
            (27.0, 0.0)
        } else {
            (19.0, -10.0)
        };
        for (count, line) in lines.split('\n').enumerate()
        {
            w.elem("text", |d| {
                d.attr("y", (count as f32 * (text_font_size * 1.2)) + text_y_pos + text_y_shift)
                    .attr("x", width / 2.0)
                    .attr("text-anchor", "middle")
                    .attr(
                        "style",
                        format!(
                            "font-family:sans-serif;\
                        font-weight:bold;\
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
        // contact avatar in middle of qrcode
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
                .attr("style", "fill:#f2f2f2");
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
            let font_offset = avatar_font_size * 0.1;
            w.elem("text", |d| {
                d.attr("y", logo_position_y + HALF_LOGO_SIZE + font_offset)
                    .attr("x", logo_position_x + HALF_LOGO_SIZE)
                    .attr("text-anchor", "middle")
                    .attr("dominant-baseline", "central")
                    .attr("alignment-baseline", "middle")
                    .attr(
                        "style",
                        format!(
                            "font-family:sans-serif;\
                            font-weight:400;\
                            font-size:{}px;\
                            fill:#ffffff;",
                            avatar_font_size
                        ),
                    );
            })
            .build(|w| {
                w.put_raw(avatar_letter.to_uppercase());
            });
        }

        // Footer logo
        const FOOTER_HEIGHT: f32 = 35.0;
        const FOOTER_WIDTH: f32 = 198.0;
        w.elem("g", |d| {
            d.attr(
                "transform",
                format!(
                    "translate({},{})",
                    (width - FOOTER_WIDTH) / 2.0,
                    height - logo_offset - FOOTER_HEIGHT - text_y_shift
                ),
            );
        })
        .build(|w| {
            w.put_raw(include_str!("../assets/qrcode_logo_footer.svg"));
        });
    });

    Ok(svg)
}
