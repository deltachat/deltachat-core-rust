Authentication-Results: bimi.icloud.com; bimi=skipped reason="insufficient dmarc"
Authentication-Results: dmarc.icloud.com; dmarc=pass header.from=fastmail.com
Authentication-Results: dkim-verifier.icloud.com;
	dkim=pass (2048-bit key) header.d=fastmail.com header.i=@fastmail.com header.b=XEFkSwVW
Authentication-Results: dkim-verifier.icloud.com;
	dkim=pass (2048-bit key) header.d=messagingengine.com header.i=@messagingengine.com header.b=tIugs7hL
Authentication-Results: spf.icloud.com; spf=pass (spf.icloud.com: domain of alice@fastmail.com designates 66.111.4.28 as permitted sender) smtp.mailfrom=alice@fastmail.com
From: <alice@fastmail.com>
To: <alice@icloud.com>
