From: <alice@icloud.com>
To: <alice@mail.ru>
Authentication-Results: mxs.mail.ru; spf=pass (mx326.i.mail.ru: domain of icloud.com designates 17.57.155.16 as permitted sender) smtp.mailfrom=alice@icloud.com smtp.helo=qs51p00im-qukt01072701.me.com;
	 dkim=pass header.d=icloud.com; dmarc=pass header.from=alice@icloud.com
