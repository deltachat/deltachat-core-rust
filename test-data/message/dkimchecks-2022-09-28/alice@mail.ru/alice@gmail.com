From: <alice@gmail.com>
To: <alice@mail.ru>
Authentication-Results: mxs.mail.ru; spf=pass (mx273.i.mail.ru: domain of gmail.com designates 209.85.221.66 as permitted sender) smtp.mailfrom=alice@gmail.com smtp.helo=mail-wr1-f66.google.com;
	 dkim=pass header.d=gmail.com; dmarc=pass header.from=alice@gmail.com
