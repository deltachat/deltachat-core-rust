ARC-Authentication-Results: i=1; mx1.messagingengine.com;
    x-csa=none;
    x-me-sender=none;
    x-ptr=pass smtp.helo=msg-1.mailo.com policy.ptr=msg-1.mailo.com;
    bimi=skipped (DMARC Policy is not at enforcement);
    arc=none (no signatures found);
    dkim=pass (1024-bit rsa key sha256) header.d=mailo.com
    header.i=@mailo.com header.b=KpmcEAgT header.a=rsa-sha256
    header.s=mailo x-bits=1024;
    dmarc=pass policy.published-domain-policy=none
    policy.applied-disposition=none policy.evaluated-disposition=none
    (p=none,d=none,d.eval=none) policy.policy-from=p
    header.from=mailo.com;
    iprev=pass smtp.remote-ip=213.182.54.11 (msg-1.mailo.com);
    spf=pass smtp.mailfrom=alice@mailo.com smtp.helo=msg-1.mailo.com
Authentication-Results: mx1.messagingengine.com;
    x-csa=none;
    x-me-sender=none;
    x-ptr=pass smtp.helo=msg-1.mailo.com policy.ptr=msg-1.mailo.com
Authentication-Results: mx1.messagingengine.com;
    bimi=skipped (DMARC Policy is not at enforcement)
Authentication-Results: mx1.messagingengine.com;
    arc=none (no signatures found)
Authentication-Results: mx1.messagingengine.com;
    dkim=pass (1024-bit rsa key sha256) header.d=mailo.com
      header.i=@mailo.com header.b=KpmcEAgT header.a=rsa-sha256
      header.s=mailo x-bits=1024;
    dmarc=pass policy.published-domain-policy=none
      policy.applied-disposition=none policy.evaluated-disposition=none
      (p=none,d=none,d.eval=none) policy.policy-from=p
      header.from=mailo.com;
    iprev=pass smtp.remote-ip=213.182.54.11 (msg-1.mailo.com);
    spf=pass smtp.mailfrom=alice@mailo.com smtp.helo=msg-1.mailo.com
From: <alice@mailo.com>
To: <alice@fastmail.com>
