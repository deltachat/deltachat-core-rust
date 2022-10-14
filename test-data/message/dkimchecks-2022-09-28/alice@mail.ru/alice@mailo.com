From: <alice@mailo.com>
To: <alice@mail.ru>
Authentication-Results: mxs.mail.ru; spf=pass (mx289.i.mail.ru: domain of mailo.com designates 213.182.54.15 as permitted sender) smtp.mailfrom=alice@mailo.com smtp.helo=msg-4.mailo.com;
	 dkim=pass header.d=mailo.com; dmarc=pass header.from=alice@mailo.com
